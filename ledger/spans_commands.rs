//! Comandos de CLI para gerenciar Spans
//!
//! Este módulo fornece comandos para criar, validar e processar
//! spans no sistema LogLine.

use clap::{Arg, ArgMatches, Command, SubCommand};
use crate::motor::span::Span;
use crate::infra::id::LogLineID;
use std::path::Path;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::io;
use chrono::{DateTime, Utc};
use serde_json::{json, Value};

/// Define os subcomandos para spans
pub fn spans_commands() -> Command<'static> {
    Command::new("spans")
        .about("Gerencia spans LogLine")
        .subcommand(
            Command::new("create")
                .about("Cria um novo span")
                .arg(Arg::new("type")
                    .short('t')
                    .long("type")
                    .help("Tipo do span")
                    .takes_value(true)
                    .required(true))
                .arg(Arg::new("content")
                    .short('c')
                    .long("content")
                    .help("Conteúdo do span em JSON")
                    .takes_value(true))
                .arg(Arg::new("content-file")
                    .short('f')
                    .long("content-file")
                    .help("Arquivo contendo o conteúdo do span em JSON")
                    .takes_value(true))
                .arg(Arg::new("output")
                    .short('o')
                    .long("output")
                    .help("Arquivo para salvar o span")
                    .takes_value(true))
                .arg(Arg::new("sign")
                    .short('s')
                    .long("sign")
                    .help("Assinar o span com uma identidade")
                    .takes_value(true))
        )
        .subcommand(
            Command::new("validate")
                .about("Valida um span")
                .arg(Arg::new("file")
                    .short('f')
                    .long("file")
                    .help("Arquivo contendo o span")
                    .takes_value(true)
                    .required(true))
                .arg(Arg::new("verbose")
                    .short('v')
                    .long("verbose")
                    .help("Exibir informações detalhadas")
                    .takes_value(false))
        )
        .subcommand(
            Command::new("display")
                .about("Exibe informações de um span")
                .arg(Arg::new("file")
                    .short('f')
                    .long("file")
                    .help("Arquivo contendo o span")
                    .takes_value(true)
                    .required(true))
                .arg(Arg::new("raw")
                    .short('r')
                    .long("raw")
                    .help("Exibir dados brutos do span")
                    .takes_value(false))
                .arg(Arg::new("validate")
                    .short('v')
                    .long("validate")
                    .help("Validar o span antes de exibir")
                    .takes_value(false))
        )
        .subcommand(
            Command::new("convert")
                .about("Converte um span entre formatos")
                .arg(Arg::new("input")
                    .short('i')
                    .long("input")
                    .help("Arquivo de entrada")
                    .takes_value(true)
                    .required(true))
                .arg(Arg::new("output")
                    .short('o')
                    .long("output")
                    .help("Arquivo de saída")
                    .takes_value(true)
                    .required(true))
                .arg(Arg::new("format")
                    .short('f')
                    .long("format")
                    .help("Formato de saída (json, binary)")
                    .takes_value(true)
                    .required(true))
        )
}

/// Executa o comando de span selecionado
pub fn run_spans_command(matches: &ArgMatches) -> Result<(), String> {
    match matches.subcommand() {
        Some(("create", create_matches)) => {
            create_span(create_matches)
        },
        Some(("validate", validate_matches)) => {
            validate_span(validate_matches)
        },
        Some(("display", display_matches)) => {
            display_span(display_matches)
        },
        Some(("convert", convert_matches)) => {
            convert_span(convert_matches)
        },
        _ => {
            Err("Comando desconhecido. Use 'logline spans --help' para ver os comandos disponíveis.".to_string())
        }
    }
}

/// Cria um novo span
fn create_span(matches: &ArgMatches) -> Result<(), String> {
    let span_type = matches.value_of("type").unwrap(); // Safe unwrap pois é required
    
    // Obter conteúdo do span
    let content = if let Some(content_str) = matches.value_of("content") {
        // Conteúdo fornecido diretamente
        serde_json::from_str::<Value>(content_str)
            .map_err(|e| format!("Erro ao processar JSON: {}", e))?
    } else if let Some(content_file) = matches.value_of("content-file") {
        // Conteúdo de um arquivo
        let content_str = fs::read_to_string(content_file)
            .map_err(|e| format!("Erro ao ler arquivo: {}", e))?;
            
        serde_json::from_str::<Value>(&content_str)
            .map_err(|e| format!("Erro ao processar JSON: {}", e))?
    } else {
        // Solicitar conteúdo
        print!("Digite o conteúdo do span em formato JSON: ");
        io::stdout().flush().map_err(|e| e.to_string())?;
        let mut content_str = String::new();
        io::stdin().read_line(&mut content_str).map_err(|e| e.to_string())?;
        
        serde_json::from_str::<Value>(&content_str)
            .map_err(|e| format!("Erro ao processar JSON: {}", e))?
    };
    
    // Gerar ID único para o span
    let span_id = format!("span_{}", uuid::Uuid::new_v4());
    
    // Criar o span
    let span = json!({
        "id": span_id,
        "type": span_type,
        "content": content,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });
    
    // Assinar o span se necessário
    let span_with_signature = if let Some(signer_id) = matches.value_of("sign") {
        // Na implementação real, aqui usaríamos a chave privada para assinar
        println!("Assinando span com a identidade: {}", signer_id);
        
        // Simular uma assinatura para demonstração
        let signature = format!("sig_{}", uuid::Uuid::new_v4());
        
        json!({
            "id": span_id,
            "type": span_type,
            "content": content,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "signer": signer_id,
            "signature": signature
        })
    } else {
        span
    };
    
    // Salvar em arquivo se necessário
    if let Some(output_file) = matches.value_of("output") {
        let span_json = serde_json::to_string_pretty(&span_with_signature)
            .map_err(|e| format!("Erro ao serializar span: {}", e))?;
            
        fs::write(output_file, span_json)
            .map_err(|e| format!("Erro ao salvar arquivo: {}", e))?;
            
        println!("Span salvo em: {}", output_file);
    } else {
        // Exibir o span criado
        let span_json = serde_json::to_string_pretty(&span_with_signature)
            .map_err(|e| format!("Erro ao serializar span: {}", e))?;
            
        println!("Span criado:");
        println!("{}", span_json);
    }
    
    Ok(())
}

/// Valida um span
fn validate_span(matches: &ArgMatches) -> Result<(), String> {
    let file_path = matches.value_of("file").unwrap(); // Safe unwrap pois é required
    let verbose = matches.is_present("verbose");
    
    // Ler o arquivo
    let span_str = fs::read_to_string(file_path)
        .map_err(|e| format!("Erro ao ler arquivo: {}", e))?;
        
    // Tentar processar como JSON
    let span_json: Value = match serde_json::from_str(&span_str) {
        Ok(json) => json,
        Err(e) => {
            return Err(format!("O arquivo não contém um JSON válido: {}", e));
        }
    };
    
    // Validar campos obrigatórios
    let validation_errors = validate_span_fields(&span_json);
    
    if validation_errors.is_empty() {
        println!("✓ Span válido!");
        
        if verbose {
            // Mostrar informações adicionais
            println!("\nDetalhes do span:");
            println!("  ID: {}", span_json["id"].as_str().unwrap_or("N/A"));
            println!("  Tipo: {}", span_json["type"].as_str().unwrap_or("N/A"));
            println!("  Timestamp: {}", span_json["timestamp"].as_str().unwrap_or("N/A"));
            
            if let Some(signer) = span_json.get("signer") {
                println!("  Assinado por: {}", signer.as_str().unwrap_or("N/A"));
                
                // Verificar assinatura
                if span_json.get("signature").is_some() {
                    println!("  Assinatura: presente (verificação não implementada nesta versão)");
                } else {
                    println!("  Aviso: span com signer mas sem assinatura!");
                }
            } else {
                println!("  Sem assinatura");
            }
        }
    } else {
        println!("✗ Span inválido!");
        println!("Erros encontrados:");
        
        for error in &validation_errors {
            println!("  - {}", error);
        }
        
        return Err("Validação falhou".to_string());
    }
    
    Ok(())
}

/// Exibe informações de um span
fn display_span(matches: &ArgMatches) -> Result<(), String> {
    let file_path = matches.value_of("file").unwrap(); // Safe unwrap pois é required
    let raw = matches.is_present("raw");
    let do_validate = matches.is_present("validate");
    
    // Ler o arquivo
    let span_str = fs::read_to_string(file_path)
        .map_err(|e| format!("Erro ao ler arquivo: {}", e))?;
        
    // Tentar processar como JSON
    let span_json: Value = match serde_json::from_str(&span_str) {
        Ok(json) => json,
        Err(e) => {
            return Err(format!("O arquivo não contém um JSON válido: {}", e));
        }
    };
    
    // Validar se solicitado
    if do_validate {
        let validation_errors = validate_span_fields(&span_json);
        
        if !validation_errors.is_empty() {
            println!("⚠️ Aviso: Span inválido!");
            println!("Erros encontrados:");
            
            for error in &validation_errors {
                println!("  - {}", error);
            }
            
            println!();
        }
    }
    
    if raw {
        // Exibir dados brutos formatados
        let formatted = serde_json::to_string_pretty(&span_json)
            .map_err(|e| format!("Erro ao formatar JSON: {}", e))?;
            
        println!("{}", formatted);
    } else {
        // Exibir informações em formato legível
        println!("Span ID: {}", span_json["id"].as_str().unwrap_or("N/A"));
        println!("Tipo: {}", span_json["type"].as_str().unwrap_or("N/A"));
        println!("Timestamp: {}", span_json["timestamp"].as_str().unwrap_or("N/A"));
        
        if let Some(signer) = span_json.get("signer") {
            println!("Assinado por: {}", signer.as_str().unwrap_or("N/A"));
            
            if let Some(signature) = span_json.get("signature") {
                println!("Assinatura: {}", signature.as_str().unwrap_or("N/A"));
            } else {
                println!("Assinatura: ausente ⚠️");
            }
        } else {
            println!("Sem assinatura");
        }
        
        println!("\nConteúdo:");
        if let Some(content) = span_json.get("content") {
            let content_str = serde_json::to_string_pretty(content)
                .map_err(|e| format!("Erro ao formatar conteúdo: {}", e))?;
                
            println!("{}", content_str);
        } else {
            println!("  <Sem conteúdo>");
        }
    }
    
    Ok(())
}

/// Converte um span entre formatos
fn convert_span(matches: &ArgMatches) -> Result<(), String> {
    let input_file = matches.value_of("input").unwrap(); // Safe unwrap pois é required
    let output_file = matches.value_of("output").unwrap(); // Safe unwrap pois é required
    let format = matches.value_of("format").unwrap(); // Safe unwrap pois é required
    
    // Ler o arquivo de entrada
    let span_str = fs::read_to_string(input_file)
        .map_err(|e| format!("Erro ao ler arquivo: {}", e))?;
        
    // Processar como JSON
    let span_json: Value = serde_json::from_str(&span_str)
        .map_err(|e| format!("O arquivo não contém um JSON válido: {}", e))?;
    
    // Converter para o formato solicitado
    match format {
        "json" => {
            // Formato já é JSON, apenas reformatar
            let formatted = serde_json::to_string_pretty(&span_json)
                .map_err(|e| format!("Erro ao formatar JSON: {}", e))?;
                
            fs::write(output_file, formatted)
                .map_err(|e| format!("Erro ao salvar arquivo: {}", e))?;
                
            println!("Span convertido para JSON e salvo em: {}", output_file);
        },
        "binary" => {
            // Em uma implementação real, converteria para um formato binário próprio
            // Por simplicidade, vamos apenas salvar como JSON compacto
            let compact = serde_json::to_string(&span_json)
                .map_err(|e| format!("Erro ao serializar JSON: {}", e))?;
                
            fs::write(output_file, compact)
                .map_err(|e| format!("Erro ao salvar arquivo: {}", e))?;
                
            println!("Span convertido para formato compacto e salvo em: {}", output_file);
            println!("Nota: Conversão para formato binário real não implementada nesta versão.");
        },
        _ => {
            return Err(format!("Formato desconhecido: {}. Formatos suportados: json, binary", format));
        }
    }
    
    Ok(())
}

/// Valida os campos obrigatórios de um span
fn validate_span_fields(span: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    
    // Validar campos obrigatórios
    if !span.get("id").is_some_and(|id| id.is_string()) {
        errors.push("Campo 'id' ausente ou não é uma string".to_string());
    }
    
    if !span.get("type").is_some_and(|t| t.is_string()) {
        errors.push("Campo 'type' ausente ou não é uma string".to_string());
    }
    
    if !span.get("timestamp").is_some_and(|ts| ts.is_string()) {
        errors.push("Campo 'timestamp' ausente ou não é uma string".to_string());
    } else {
        // Validar formato do timestamp
        let ts_str = span["timestamp"].as_str().unwrap();
        if chrono::DateTime::parse_from_rfc3339(ts_str).is_err() {
            errors.push(format!("Timestamp '{}' não está no formato RFC3339", ts_str));
        }
    }
    
    if !span.get("content").is_some() {
        errors.push("Campo 'content' ausente".to_string());
    }
    
    // Validar coerência entre signer e signature
    if span.get("signer").is_some() && !span.get("signature").is_some() {
        errors.push("Span possui 'signer' mas não possui 'signature'".to_string());
    }
    
    if span.get("signature").is_some() && !span.get("signer").is_some() {
        errors.push("Span possui 'signature' mas não possui 'signer'".to_string());
    }
    
    errors
}