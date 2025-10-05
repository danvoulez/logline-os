use std::env;

use base64::{engine::general_purpose, Engine as _};
use chrono::Utc;
use clap::{Args, Parser, Subcommand};
use logline_core::identity::LogLineKeyPair;

mod onboarding;

use onboarding::{
    print_assignment, print_identity_created, print_purpose, print_shell_execution,
    print_template_selected, print_tenant_created, slugify, AssignIdentityRequest,
    CreateIdentityRequest, CreateTenantRequest, DeclarePurposeRequest, ExecuteShellRequest,
    OnboardingCliError, OnboardingClient, SessionStore, StoredSession,
};

#[derive(Parser)]
#[command(name = "logline")]
#[command(about = "LogLine Universe - Distributed logging and identity system", long_about = None)]
struct Cli {
    #[arg(long, global = true, env = "LOGLINE_GATEWAY_URL")]
    gateway: Option<String>,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create resources required to join the LogLine Universe
    #[command(subcommand)]
    Create(CreateCommands),
    /// Assign identities to entities
    #[command(subcommand)]
    Assign(AssignCommands),
    /// Initialise computable applications
    #[command(subcommand)]
    Init(InitCommands),
    /// Declare computable intents
    #[command(subcommand)]
    Declare(DeclareCommands),
    /// Execute computable actions through the gateway
    #[command(subcommand)]
    Run(RunCommands),
    /// Generate a standalone LogLine ID locally
    GenerateId {
        /// Node name for the identity
        #[arg(short, long)]
        node_name: String,
    },
    /// Show version information
    Version,
}

#[derive(Subcommand)]
enum CreateCommands {
    /// Create a new computable identity
    Identity(CreateIdentityArgs),
    /// Create a new tenant/organisation
    Tenant(CreateTenantArgs),
}

#[derive(Args)]
struct CreateIdentityArgs {
    #[arg(long)]
    name: String,
    #[arg(long)]
    handle: String,
    #[arg(long, default_value_t = false)]
    ghost: bool,
}

#[derive(Args)]
struct CreateTenantArgs {
    #[arg(long)]
    name: String,
    /// Use a specific identity handle (defaults to active session)
    #[arg(long)]
    identity: Option<String>,
}

#[derive(Subcommand)]
enum AssignCommands {
    /// Assign a LogLine identity to a tenant
    Identity(AssignIdentityArgs),
}

#[derive(Args)]
struct AssignIdentityArgs {
    /// Handle of the identity being assigned
    handle: String,
    /// Target entity (e.g. "tenant voulezvous")
    #[arg(long = "to")]
    target: String,
}

#[derive(Subcommand)]
enum InitCommands {
    /// Initialise an application template for the tenant
    App(InitAppArgs),
}

#[derive(Args)]
struct InitAppArgs {
    #[arg(long)]
    template: String,
    #[arg(long)]
    owner: Option<String>,
    #[arg(long)]
    identity: Option<String>,
}

#[derive(Subcommand)]
enum DeclareCommands {
    /// Declare a computable purpose for the initial application
    Purpose(DeclarePurposeArgs),
}

#[derive(Args)]
struct DeclarePurposeArgs {
    #[arg(long)]
    app: String,
    #[arg(long)]
    description: String,
    #[arg(long)]
    identity: Option<String>,
}

#[derive(Subcommand)]
enum RunCommands {
    /// Run a natural language shell command inside the onboarding context
    Shell(RunShellArgs),
}

#[derive(Args)]
struct RunShellArgs {
    #[arg(long)]
    identity: Option<String>,
    #[arg(short = 'c', long = "command")]
    command: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), OnboardingCliError> {
    let cli = Cli::parse();

    match cli.command {
        Commands::GenerateId { node_name } => {
            let keypair = LogLineKeyPair::generate(&node_name, None, None, false);
            println!(
                "Generated LogLine ID: {}",
                keypair
                    .id
                    .to_json()
                    .unwrap_or_else(|_| "Error serializing ID".to_string())
            );
            println!(
                "Public Key: {}",
                general_purpose::STANDARD.encode(keypair.public_key_bytes())
            );
            Ok(())
        }
        Commands::Version => {
            println!("LogLine Universe v{}", env!("CARGO_PKG_VERSION"));
            println!("Microservices architecture with WebSocket mesh");
            Ok(())
        }
        command => {
            let base_url = cli
                .gateway
                .or_else(|| env::var("LOGLINE_GATEWAY_URL").ok())
                .unwrap_or_else(|| "http://127.0.0.1:8070".to_string());
            let client = OnboardingClient::new(&base_url)?;
            let mut store = SessionStore::load()?;

            match command {
                Commands::Create(CreateCommands::Identity(args)) => {
                    let request = CreateIdentityRequest {
                        name: args.name.clone(),
                        handle: args.handle.clone(),
                        ghost: args.ghost,
                    };
                    let response = client.create_identity(request).await?;
                    print_identity_created(&response);
                    let session = StoredSession::new(
                        &response.handle,
                        response.session_id,
                        response.identity.logline_id.clone(),
                        response.identity.signing_key.clone(),
                    );
                    store.upsert(session);
                    store.save()?;
                }
                Commands::Create(CreateCommands::Tenant(args)) => {
                    let handle = args
                        .identity
                        .clone()
                        .or_else(|| store.active_session_id().ok().map(|(h, _)| h))
                        .ok_or(OnboardingCliError::NoActiveSession)?;
                    let session = store.session(&handle)?;
                    let request = CreateTenantRequest {
                        session_id: session.session_id,
                        name: args.name.clone(),
                    };
                    let response = client.create_tenant(request).await?;
                    print_tenant_created(&response, &args.name);
                    let session = store.session_mut(&handle)?;
                    session.tenant_id = Some(response.tenant_id.clone());
                    session.updated_at = Utc::now();
                    store.save()?;
                }
                Commands::Assign(AssignCommands::Identity(args)) => {
                    let mut parts = args.target.split_whitespace();
                    let kind = parts.next().unwrap_or("");
                    let target_value = parts.collect::<Vec<_>>().join(" ");
                    if kind != "tenant" {
                        return Err(OnboardingCliError::Validation(
                            "apenas atribuição para tenant é suportada".into(),
                        ));
                    }
                    if target_value.is_empty() {
                        return Err(OnboardingCliError::Validation(
                            "especifique o tenant após '--to'".into(),
                        ));
                    }
                    let tenant_id = slugify(&target_value);
                    let session = store.session(&args.handle)?;
                    let request = AssignIdentityRequest {
                        session_id: session.session_id,
                        handle: args.handle.clone(),
                        tenant_id: tenant_id.clone(),
                    };
                    let response = client.assign_identity(request).await?;
                    print_assignment(&response, &args.handle);
                    let session = store.session_mut(&args.handle)?;
                    session.tenant_id = Some(response.tenant_id.clone());
                    session.jwt = Some(response.jwt.clone());
                    session.signing_key = response.signing_key.clone();
                    session.updated_at = Utc::now();
                    store.set_active(&args.handle)?;
                    store.save()?;
                }
                Commands::Init(InitCommands::App(args)) => {
                    let handle = args
                        .identity
                        .clone()
                        .or_else(|| store.active_session_id().ok().map(|(h, _)| h))
                        .ok_or(OnboardingCliError::NoActiveSession)?;
                    let session = store.session(&handle)?;
                    let request = onboarding::SelectTemplateRequest {
                        session_id: session.session_id,
                        template: args.template.clone(),
                        owner: args.owner.clone(),
                    };
                    let response = client.select_template(request).await?;
                    print_template_selected(&response);
                    store.save()?;
                }
                Commands::Declare(DeclareCommands::Purpose(args)) => {
                    let handle = args
                        .identity
                        .clone()
                        .or_else(|| store.active_session_id().ok().map(|(h, _)| h))
                        .ok_or(OnboardingCliError::NoActiveSession)?;
                    let session = store.session(&handle)?;
                    let request = DeclarePurposeRequest {
                        session_id: session.session_id,
                        app: args.app.clone(),
                        description: args.description.clone(),
                    };
                    let response = client.declare_purpose(request).await?;
                    print_purpose(&response);
                    store.save()?;
                }
                Commands::Run(RunCommands::Shell(args)) => {
                    let handle = args
                        .identity
                        .clone()
                        .or_else(|| store.active_session_id().ok().map(|(h, _)| h))
                        .ok_or(OnboardingCliError::NoActiveSession)?;
                    let session = store.session(&handle)?;
                    let command = if let Some(cmd) = args.command.clone() {
                        cmd
                    } else {
                        onboarding::read_shell_command("> ").await?
                    };
                    let request = ExecuteShellRequest {
                        session_id: session.session_id,
                        command: command.clone(),
                    };
                    let response = client.execute_shell(request).await?;
                    print_shell_execution(&response, &command);
                    store.save()?;
                }
                _ => {}
            }

            Ok(())
        }
    }
}
