use std::net::SocketAddr;
use std::time::Duration;

use anyhow::anyhow;
use axum::routing::get;
use axum::{Json, Router};
use futures::{SinkExt, StreamExt};
use logline_core::websocket::{ServiceMessage, WebSocketEnvelope};
use logline_gateway::config::{GatewayConfig, ResilienceConfig, SecurityConfig, ServiceUrls};
use logline_gateway::start_gateway;
use serde_json::json;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot};
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message as WsMessage;

struct HttpService {
    addr: SocketAddr,
    shutdown: oneshot::Sender<()>,
}

async fn spawn_http_service(router: Router) -> anyhow::Result<HttpService> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let (tx, rx) = oneshot::channel();
    tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async move {
                let _ = rx.await;
            })
            .await
            .ok();
    });

    Ok(HttpService { addr, shutdown: tx })
}

struct MeshPeer {
    url: String,
    sender: mpsc::UnboundedSender<ServiceMessage>,
    receiver: mpsc::UnboundedReceiver<ServiceMessage>,
    shutdown: oneshot::Sender<()>,
}

async fn spawn_mesh_peer(name: &'static str) -> anyhow::Result<MeshPeer> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let url = format!("ws://{}", addr);

    let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
    let (to_gateway_tx, mut to_gateway_rx) = mpsc::unbounded_channel();
    let (from_gateway_tx, from_gateway_rx) = mpsc::unbounded_channel();

    tokio::spawn(async move {
        let (stream, _) = match listener.accept().await {
            Ok(accepted) => accepted,
            Err(err) => {
                eprintln!("failed to accept mesh peer: {err}");
                return;
            }
        };

        let ws_stream = match tokio_tungstenite::accept_async(stream).await {
            Ok(stream) => stream,
            Err(err) => {
                eprintln!("failed to perform websocket handshake: {err}");
                return;
            }
        };
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // Expect ServiceHello from gateway.
        if let Some(Ok(message)) = ws_receiver.next().await {
            if let Ok(service_message) = decode_message(message) {
                let _ = from_gateway_tx.send(service_message);
            }
        }

        let hello = ServiceMessage::ServiceHello {
            sender: name.to_string(),
            capabilities: vec![],
        };
        if let Err(err) = send_message(&mut ws_sender, &hello).await {
            eprintln!("failed to send hello from mock peer: {err}");
            return;
        }

        loop {
            tokio::select! {
                Some(msg) = ws_receiver.next() => {
                    match msg {
                        Ok(WsMessage::Text(text)) => {
                            match decode_text(&text) {
                                Ok(service_msg) => {
                                    let _ = from_gateway_tx.send(service_msg);
                                }
                                Err(err) => {
                                    eprintln!("failed to decode message from gateway: {err}");
                                }
                            }
                        }
                        Ok(WsMessage::Binary(bytes)) => {
                            match decode_bytes(&bytes) {
                                Ok(service_msg) => {
                                    let _ = from_gateway_tx.send(service_msg);
                                }
                                Err(err) => {
                                    eprintln!("failed to decode message from gateway: {err}");
                                }
                            }
                        }
                        Ok(WsMessage::Ping(payload)) => {
                            let _ = ws_sender.send(WsMessage::Pong(payload)).await;
                        }
                        Ok(WsMessage::Pong(_)) => {}
                        Ok(WsMessage::Close(_)) => break,
                        Ok(WsMessage::Frame(_)) => {}
                        Err(err) => {
                            eprintln!("mesh connection error: {err}");
                            break;
                        }
                    }
                }
                Some(outgoing) = to_gateway_rx.recv() => {
                    if let Err(err) = send_message(&mut ws_sender, &outgoing).await {
                        eprintln!("failed to send message from mock peer: {err}");
                        break;
                    }
                }
                _ = &mut shutdown_rx => {
                    break;
                }
            }
        }
    });

    Ok(MeshPeer {
        url,
        sender: to_gateway_tx,
        receiver: from_gateway_rx,
        shutdown: shutdown_tx,
    })
}

async fn send_message(
    sender: &mut futures::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
        WsMessage,
    >,
    message: &ServiceMessage,
) -> anyhow::Result<()> {
    let envelope = WebSocketEnvelope::from_service_message(message)?;
    let serialized = serde_json::to_string(&envelope)?;
    sender.send(WsMessage::Text(serialized)).await?;
    Ok(())
}

fn decode_message(message: WsMessage) -> anyhow::Result<ServiceMessage> {
    match message {
        WsMessage::Text(text) => decode_text(&text),
        WsMessage::Binary(bytes) => decode_bytes(&bytes),
        other => anyhow::bail!("unsupported message: {other:?}"),
    }
}

fn decode_text(text: &str) -> anyhow::Result<ServiceMessage> {
    let envelope: WebSocketEnvelope = serde_json::from_str(text)?;
    Ok(envelope.into_service_message()?)
}

fn decode_bytes(bytes: &[u8]) -> anyhow::Result<ServiceMessage> {
    let envelope: WebSocketEnvelope = serde_json::from_slice(bytes)?;
    Ok(envelope.into_service_message()?)
}

struct FlakyPeer {
    url: String,
    connections: mpsc::UnboundedReceiver<usize>,
    shutdown: oneshot::Sender<()>,
}

async fn spawn_flaky_peer(name: &'static str) -> anyhow::Result<FlakyPeer> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let url = format!("ws://{}", addr);
    let (conn_tx, conn_rx) = mpsc::unbounded_channel();
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel();

    tokio::spawn(async move {
        let mut counter: usize = 0;
        loop {
            tokio::select! {
                accept = listener.accept() => {
                    let (stream, _) = match accept {
                        Ok(value) => value,
                        Err(err) => {
                            eprintln!("failed to accept flaky peer connection: {err}");
                            break;
                        }
                    };

                    counter += 1;
                    let mut tx_clone = conn_tx.clone();
                    tokio::spawn(async move {
                        if let Err(err) = handle_flaky_connection(stream, name, counter, &mut tx_clone).await {
                            eprintln!("flaky peer connection error: {err}");
                        }
                    });
                }
                _ = &mut shutdown_rx => {
                    break;
                }
            }
        }
    });

    Ok(FlakyPeer {
        url,
        connections: conn_rx,
        shutdown: shutdown_tx,
    })
}

async fn handle_flaky_connection(
    stream: tokio::net::TcpStream,
    name: &str,
    attempt: usize,
    conn_tx: &mut mpsc::UnboundedSender<usize>,
) -> anyhow::Result<()> {
    let ws_stream = tokio_tungstenite::accept_async(stream).await?;
    let (mut sender, mut receiver) = ws_stream.split();
    conn_tx.send(attempt).ok();

    if let Some(Ok(message)) = receiver.next().await {
        let _ = decode_message(message)?;
    }

    let hello = ServiceMessage::ServiceHello {
        sender: format!("{name}-flaky"),
        capabilities: vec![],
    };
    send_message(&mut sender, &hello).await?;
    tokio::time::sleep(Duration::from_millis(100)).await;
    // Drop connection immediately to force reconnect.
    drop(sender);
    drop(receiver);
    Ok(())
}

fn security_config_for_tests() -> SecurityConfig {
    SecurityConfig {
        jwt_secret: "test-secret".into(),
        jwt_issuer: "tests".into(),
        jwt_audience: "tests".into(),
        rate_limit_per_minute: 10_000,
        max_concurrent_requests: 256,
        cors_allowed_origins: vec!["*".into()],
        cors_allow_credentials: false,
        service_token: None,
        public_paths: vec!["/healthz".into()],
    }
}

fn resilience_config_for_tests() -> ResilienceConfig {
    ResilienceConfig {
        request_timeout: Duration::from_secs(5),
        circuit_breaker_threshold: 5,
        circuit_breaker_reset: Duration::from_secs(1),
        retry_attempts: 1,
        retry_backoff: Duration::from_millis(50),
        dead_letter_capacity: 32,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rest_proxy_forwards_engine_ping() -> anyhow::Result<()> {
    let engine_service = spawn_http_service(
        Router::new()
            .route("/ping", get(|| async { "pong" }))
            .route("/health", get(|| async { "ok" })),
    )
    .await?;

    let rest_base = format!("http://{}", engine_service.addr);

    let config = GatewayConfig {
        bind_address: "127.0.0.1:0".into(),
        engine: ServiceUrls::new("engine", "logline-engine", rest_base.clone(), None),
        rules: ServiceUrls::new("rules", "logline-rules", rest_base.clone(), None),
        timeline: ServiceUrls::new("timeline", "logline-timeline", rest_base.clone(), None),
        identity: ServiceUrls::new("id", "logline-id", rest_base.clone(), None),
        federation: ServiceUrls::new("federation", "logline-federation", rest_base.clone(), None),
        security: security_config_for_tests(),
        resilience: resilience_config_for_tests(),
        tls: None,
    };

    let gateway = start_gateway(config).await?;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://{}/engine/ping", gateway.addr))
        .send()
        .await?
        .text()
        .await?;
    assert_eq!(response, "pong");

    gateway.shutdown();
    let _ = engine_service.shutdown.send(());

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn websocket_span_created_forwarded_to_rules() -> anyhow::Result<()> {
    let mut timeline_peer = spawn_mesh_peer("logline-timeline").await?;
    let mut rules_peer = spawn_mesh_peer("logline-rules").await?;

    let rest_base = "http://127.0.0.1:3999".to_string();

    let config = GatewayConfig {
        bind_address: "127.0.0.1:0".to_string(),
        engine: ServiceUrls::new("engine", "logline-engine", rest_base.clone(), None),
        rules: ServiceUrls::new(
            "rules",
            "logline-rules",
            rest_base.clone(),
            Some(rules_peer.url.clone()),
        ),
        timeline: ServiceUrls::new(
            "timeline",
            "logline-timeline",
            rest_base.clone(),
            Some(timeline_peer.url.clone()),
        ),
        identity: ServiceUrls::new("id", "logline-id", rest_base.clone(), None),
        federation: ServiceUrls::new("federation", "logline-federation", rest_base.clone(), None),
        security: security_config_for_tests(),
        resilience: resilience_config_for_tests(),
        tls: None,
    };

    let gateway = start_gateway(config).await?;

    // Await handshake messages from gateway.
    let hello_from_gateway = timeout(Duration::from_secs(2), timeline_peer.receiver.recv())
        .await?
        .ok_or_else(|| anyhow!("gateway did not send hello to timeline"))?;
    assert!(matches!(
        hello_from_gateway,
        ServiceMessage::ServiceHello { .. }
    ));
    let rules_hello = timeout(Duration::from_secs(2), rules_peer.receiver.recv())
        .await?
        .ok_or_else(|| anyhow!("gateway did not send hello to rules"))?;
    assert!(matches!(rules_hello, ServiceMessage::ServiceHello { .. }));

    let span_message = ServiceMessage::SpanCreated {
        span_id: "abc123".into(),
        tenant_id: Some("tenant".into()),
        span: json!({"id": "abc123"}),
        metadata: json!({"source": "test"}),
    };
    timeline_peer.sender.send(span_message.clone())?;

    let forwarded = timeout(Duration::from_secs(2), rules_peer.receiver.recv())
        .await?
        .ok_or_else(|| anyhow!("rules peer did not receive forwarded span"))?;
    match forwarded {
        ServiceMessage::SpanCreated { span_id, .. } => {
            assert_eq!(span_id, "abc123");
        }
        other => panic!("unexpected message forwarded to rules: {other:?}"),
    }

    gateway.shutdown();
    let _ = timeline_peer.shutdown.send(());
    let _ = rules_peer.shutdown.send(());

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn health_check_reports_services_and_mesh() -> anyhow::Result<()> {
    let http_service = spawn_http_service(
        Router::new().route("/health", get(|| async { Json(json!({"status": "ok"})) })),
    )
    .await?;
    let rest_base = format!("http://{}", http_service.addr);

    let mut timeline_peer = spawn_mesh_peer("logline-timeline").await?;
    let mut rules_peer = spawn_mesh_peer("logline-rules").await?;

    let config = GatewayConfig {
        bind_address: "127.0.0.1:0".into(),
        engine: ServiceUrls::new("engine", "logline-engine", rest_base.clone(), None),
        rules: ServiceUrls::new(
            "rules",
            "logline-rules",
            rest_base.clone(),
            Some(rules_peer.url.clone()),
        ),
        timeline: ServiceUrls::new(
            "timeline",
            "logline-timeline",
            rest_base.clone(),
            Some(timeline_peer.url.clone()),
        ),
        identity: ServiceUrls::new("id", "logline-id", rest_base.clone(), None),
        federation: ServiceUrls::new("federation", "logline-federation", rest_base.clone(), None),
        security: security_config_for_tests(),
        resilience: resilience_config_for_tests(),
        tls: None,
    };

    let gateway = start_gateway(config).await?;

    // consume handshake messages so peers remain connected.
    let _ = timeline_peer.receiver.recv().await;
    let _ = rules_peer.receiver.recv().await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://{}/healthz", gateway.addr))
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    assert_eq!(response["status"], "ok");
    assert_eq!(response["services"].as_array().map(|a| a.len()), Some(5));
    let connected = response["mesh"]["connected"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert!(connected.iter().any(|v| v == "logline-timeline"));
    assert!(connected.iter().any(|v| v == "logline-rules"));

    gateway.shutdown();
    let _ = http_service.shutdown.send(());
    let _ = timeline_peer.shutdown.send(());
    let _ = rules_peer.shutdown.send(());

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mesh_reconnects_to_flaky_peer() -> anyhow::Result<()> {
    let http_service =
        spawn_http_service(Router::new().route("/health", get(|| async { "ok" }))).await?;
    let rest_base = format!("http://{}", http_service.addr);

    let mut flaky_peer = spawn_flaky_peer("logline-timeline").await?;

    let config = GatewayConfig {
        bind_address: "127.0.0.1:0".into(),
        engine: ServiceUrls::new("engine", "logline-engine", rest_base.clone(), None),
        rules: ServiceUrls::new("rules", "logline-rules", rest_base.clone(), None),
        timeline: ServiceUrls::new(
            "timeline",
            "logline-timeline",
            rest_base.clone(),
            Some(flaky_peer.url.clone()),
        ),
        identity: ServiceUrls::new("id", "logline-id", rest_base.clone(), None),
        federation: ServiceUrls::new("federation", "logline-federation", rest_base.clone(), None),
        security: security_config_for_tests(),
        resilience: resilience_config_for_tests(),
        tls: None,
    };

    let gateway = start_gateway(config).await?;

    let first = timeout(Duration::from_secs(2), flaky_peer.connections.recv())
        .await?
        .ok_or_else(|| anyhow!("flaky peer never accepted initial connection"))?;
    assert_eq!(first, 1);
    let second = timeout(Duration::from_secs(5), flaky_peer.connections.recv())
        .await?
        .ok_or_else(|| anyhow!("flaky peer never saw reconnection"))?;
    assert_eq!(second, 2);

    gateway.shutdown();
    let _ = http_service.shutdown.send(());
    let _ = flaky_peer.shutdown.send(());

    Ok(())
}
