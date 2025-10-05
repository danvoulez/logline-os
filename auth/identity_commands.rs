//! Comandos de CLI para gerenciar LogLine IDs
//!
//! Este módulo fornece comandos para criar, importar, exportar e
//! gerenciar identidades LogLine através da linha de comando.

use clap::{Arg, ArgMatches, Command, SubCommand};
use crate::infra::id::{LogLineID, LogLineKeyPair, LogLineIDBuilder};
use std::path::Path;
use std::fs::{self, File};
use std::io::{Read, Write};
use rpassword::read_password;
use std::io::{self, BufReader};
use serde_json::{json, Value};

/// Define os subcomandos para identidades
pub fn id_commands() -> Command<'static> {
    Command::new("id")
        .about("Gerencia identidades LogLine")
        .subcommand(
            Command::new("new")
                .about("Cria uma nova identidade LogLine")
                .arg(Arg::new("alias")
                    .short('a')
                    .long("alias")
                    .help("Alias para a identidade")
                    .takes_value(true))
                .arg(Arg::new("tenant")
                    .short('t')
                    .long("tenant")
                    .help("ID do tenant")
                    .takes_value(true))
                .arg(Arg::new("org")
                    .short('o')
                    .long("org")
                    .help("Criar como identidade organizacional")
                    .takes_value(false))
                .arg(Arg::new("output")
                    .short('f')
                    .long("output")
                    .help("Arquivo para salvar a identidade")
                    .takes_value(true))
                .arg(Arg::new("password")
                    .short('p')
                    .long("password")
                    .help("Proteger com senha (será solicitada)")
                    .takes_value(false))
        )
        .subcommand(
            Command::new("import")
                .about("Importa uma identidade LogLine existente")
                .arg(Arg::new("file")
                    .short('f')
                    .long("file")
                    .help("Arquivo contendo a identidade")
                    .takes_value(true)
                    .required(true))
                .arg(Arg::new("password")
                    .short('p')
                    .long("password")
                    .help("Senha para descriptografar (será solicitada)")
                    .takes_value(false))
        )
        .subcommand(
            Command::new("export")
                .about("Exporta uma identidade LogLine")
                .arg(Arg::new("id")
                    .short('i')
                    .long("id")
                    .help("ID da identidade a exportar")
                    .takes_value(true)
                    .required(true))
                .arg(Arg::new("file")
                    .short('f')
                    .long("file")
                    .help("Arquivo para salvar a identidade exportada")
                    .takes_value(true)
                    .required(true))
                .arg(Arg::new("password")
                    .short('p')
                    .long("password")
                    .help("Proteger com senha (será solicitada)")
                    .takes_value(false))
                .arg(Arg::new("public-only")
                    .long("public-only")
                    .help("Exportar apenas chave pública")
                    .takes_value(false))
        )
        .subcommand(
            Command::new("list")
                .about("Lista identidades disponíveis")
                .arg(Arg::new("tenant")
                    .short('t')
                    .long("tenant")
                    .help("Filtrar por tenant")
                    .takes_value(true))
                .arg(Arg::new("org-only")
                    .long("org-only")
                    .help("Mostrar apenas organizações")
                    .takes_value(false))
                .arg(Arg::new("user-only")
                    .long("user-only")
                    .help("Mostrar apenas usuários")
                    .takes_value(false))
        )
}

/// Executa o comando de identidade selecionado
pub fn run_id_command(matches: &ArgMatches) -> Result<(), String> {
    match matches.subcommand() {
        Some(("new", new_matches)) => {
            create_new_id(new_matches)
        },
        Some(("import", import_matches)) => {
            import_id(import_matches)
        },
        Some(("export", export_matches)) => {
            export_id(export_matches)
        },
        Some(("list", list_matches)) => {
            list_ids(list_matches)
        },
        _ => {
            Err("Comando desconhecido. Use 'logline id --help' para ver os comandos disponíveis.".to_string())
        }
    }
}

/// Cria uma nova identidade
fn create_new_id(matches: &ArgMatches) -> Result<(), String> {
    // Obter parâmetros
    let alias = matches.value_of("alias").map(|s| s.to_string());
    let tenant_id = matches.value_of("tenant").map(|s| s.to_string());
    let is_org = matches.is_present("org");
    
    // Criar keypair
    let keypair = if is_org {
        LogLineIDBuilder::new_organization(alias.clone(), tenant_id.clone())
    } else {
        LogLineIDBuilder::new_user(alias.clone(), tenant_id.clone())
    };
    
    // Exibir informações
    println!("Nova identidade criada:");
    println!("  ID: {}", keypair.id.display_name());
    println!("  Chave pública: {}", keypair.id.public_key);
    if let Some(tenant) = &keypair.id.tenant_id {
        println!("  Tenant: {}", tenant);
    }
    println!("  Tipo: {}", if is_org { "Organização" } else { "Usuário" });
    
    // Salvar se necessário
    if let Some(output_file) = matches.value_of("output") {
        let use_password = matches.is_present("password");
        let password = if use_password {
            print!("Digite uma senha para proteger a chave privada: ");
            io::stdout().flush().map_err(|e| e.to_string())?;
            let password = read_password().map_err(|e| e.to_string())?;
            Some(password)
        } else {
            None
        };
        
        save_keypair_to_file(&keypair, output_file, password.as_deref())?;
        println!("Identidade salva em: {}", output_file);
    } else {
        println!("\nAviso: A identidade não foi salva. Use a opção --output para salvá-la.");
    }
    
    Ok(())
}

/// Importa uma identidade de um arquivo
fn import_id(matches: &ArgMatches) -> Result<(), String> {
    let file_path = matches.value_of("file").unwrap(); // Safe unwrap pois é required
    
    // Ler arquivo
    let mut file = File::open(file_path)
        .map_err(|e| format!("Erro ao abrir arquivo: {}", e))?;
        
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .map_err(|e| format!("Erro ao ler arquivo: {}", e))?;
        
    // Deserializar JSON
    let data: Value = serde_json::from_str(&contents)
        .map_err(|e| format!("Erro ao ler JSON: {}", e))?;
        
    let requires_password = data.get("encrypted_key").is_some();
    
    let keypair = if requires_password {
        // Obter senha
        print!("Digite a senha para descriptografar a chave: ");
        io::stdout().flush().map_err(|e| e.to_string())?;
        let password = read_password().map_err(|e| e.to_string())?;
        
        let encrypted_key = data["encrypted_key"].as_str()
            .ok_or_else(|| "Chave privada não encontrada no arquivo".to_string())?;
            
        let alias = data.get("alias").and_then(|a| a.as_str()).map(|s| s.to_string());
        let tenant_id = data.get("tenant_id").and_then(|t| t.as_str()).map(|s| s.to_string());
        let is_org = data.get("is_org").and_then(|o| o.as_bool()).unwrap_or(false);
        
        // Importar com senha
        LogLineKeyPair::import_secret_key(encrypted_key, &password, alias, tenant_id, is_org)?
    } else {
        return Err("Este arquivo contém apenas a chave pública, não é possível importar a identidade completa.".to_string());
    };
    
    println!("Identidade importada com sucesso:");
    println!("  ID: {}", keypair.id.display_name());
    println!("  Chave pública: {}", keypair.id.public_key);
    if let Some(tenant) = &keypair.id.tenant_id {
        println!("  Tenant: {}", tenant);
    }
    println!("  Tipo: {}", if keypair.id.is_org { "Organização" } else { "Usuário" });
    
    Ok(())
}

/// Exporta uma identidade para um arquivo
fn export_id(matches: &ArgMatches) -> Result<(), String> {
    let id_value = matches.value_of("id").unwrap(); // Safe unwrap pois é required
    let output_file = matches.value_of("file").unwrap(); // Safe unwrap pois é required
    let public_only = matches.is_present("public-only");
    
    // Aqui normalmente buscaríamos a identidade do armazenamento
    // Por simplicidade, vamos criar uma nova para demonstração
    let keypair = LogLineKeyPair::generate(
        Some("Demo Export".to_string()),
        Some("tenant-demo".to_string()),
        false,
    );
    
    if public_only {
        // Exportar apenas chave pública
        let public_json = json!({
            "public_key": keypair.id.public_key,
            "alias": keypair.id.alias,
            "tenant_id": keypair.id.tenant_id,
            "is_org": keypair.id.is_org,
            "metadata": keypair.id.metadata,
            "exported_at": chrono::Utc::now().to_rfc3339(),
        });
        
        let json_str = serde_json::to_string_pretty(&public_json)
            .map_err(|e| format!("Erro ao serializar JSON: {}", e))?;
            
        fs::write(output_file, json_str)
            .map_err(|e| format!("Erro ao salvar arquivo: {}", e))?;
            
        println!("Chave pública exportada para: {}", output_file);
    } else {
        // Exportar chave privada com senha
        print!("Digite uma senha para proteger a chave privada: ");
        io::stdout().flush().map_err(|e| e.to_string())?;
        let password = read_password().map_err(|e| e.to_string())?;
        
        // Validar senha
        if password.len() < 8 {
            return Err("A senha deve ter pelo menos 8 caracteres".to_string());
        }
        
        // Confirmar senha
        print!("Confirme a senha: ");
        io::stdout().flush().map_err(|e| e.to_string())?;
        let confirm_password = read_password().map_err(|e| e.to_string())?;
        
        if password != confirm_password {
            return Err("As senhas não coincidem".to_string());
        }
        
        // Exportar chave privada criptografada
        let encrypted_key = keypair.export_secret_key(&password)?;
        
        let full_json = json!({
            "public_key": keypair.id.public_key,
            "alias": keypair.id.alias,
            "tenant_id": keypair.id.tenant_id,
            "is_org": keypair.id.is_org,
            "metadata": keypair.id.metadata,
            "encrypted_key": encrypted_key,
            "exported_at": chrono::Utc::now().to_rfc3339(),
        });
        
        let json_str = serde_json::to_string_pretty(&full_json)
            .map_err(|e| format!("Erro ao serializar JSON: {}", e))?;
            
        fs::write(output_file, json_str)
            .map_err(|e| format!("Erro ao salvar arquivo: {}", e))?;
            
        println!("Identidade completa exportada para: {}", output_file);
        println!("IMPORTANTE: Guarde a senha em local seguro. Se perdê-la, não será possível recuperar a chave privada.");
    }
    
    Ok(())
}

/// Lista identidades disponíveis
fn list_ids(matches: &ArgMatches) -> Result<(), String> {
    // Aqui normalmente buscaríamos as identidades do armazenamento
    // Por simplicidade, vamos criar algumas para demonstração
    let demo_ids = vec![
        LogLineIDBuilder::new_user(Some("Alice".to_string()), Some("tenant1".to_string())).id,
        LogLineIDBuilder::new_user(Some("Bob".to_string()), Some("tenant1".to_string())).id,
        LogLineIDBuilder::new_organization(Some("Empresa A".to_string()), Some("tenant1".to_string())).id,
        LogLineIDBuilder::new_user(Some("Carlos".to_string()), Some("tenant2".to_string())).id,
        LogLineIDBuilder::new_organization(Some("Empresa B".to_string()), Some("tenant2".to_string())).id,
    ];
    
    // Aplicar filtros
    let tenant_filter = matches.value_of("tenant");
    let org_only = matches.is_present("org-only");
    let user_only = matches.is_present("user-only");
    
    let filtered_ids: Vec<&LogLineID> = demo_ids.iter()
        .filter(|id| {
            // Filtrar por tenant se especificado
            if let Some(tenant) = tenant_filter {
                if let Some(id_tenant) = &id.tenant_id {
                    if id_tenant != tenant {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            
            // Filtrar por tipo
            if org_only && !id.is_org {
                return false;
            }
            
            if user_only && id.is_org {
                return false;
            }
            
            true
        })
        .collect();
    
    // Exibir resultados
    if filtered_ids.is_empty() {
        println!("Nenhuma identidade encontrada com os filtros especificados.");
        return Ok(());
    }
    
    println!("Identidades encontradas:");
    for (idx, id) in filtered_ids.iter().enumerate() {
        println!("{}. {} ({})", idx + 1, id.display_name(), 
            if let Some(tenant) = &id.tenant_id {
                format!("tenant: {}", tenant)
            } else {
                "sem tenant".to_string()
            });
    }
    
    Ok(())
}

/// Salva um par de chaves em um arquivo
fn save_keypair_to_file(keypair: &LogLineKeyPair, file_path: &str, password: Option<&str>) -> Result<(), String> {
    let json_value = if let Some(password) = password {
        // Exportar chave privada criptografada
        let encrypted_key = keypair.export_secret_key(password)?;
        
        json!({
            "public_key": keypair.id.public_key,
            "alias": keypair.id.alias,
            "tenant_id": keypair.id.tenant_id,
            "is_org": keypair.id.is_org,
            "metadata": keypair.id.metadata,
            "encrypted_key": encrypted_key,
            "created_at": chrono::Utc::now().to_rfc3339(),
        })
    } else {
        // Exportar apenas chave pública
        json!({
            "public_key": keypair.id.public_key,
            "alias": keypair.id.alias,
            "tenant_id": keypair.id.tenant_id,
            "is_org": keypair.id.is_org,
            "metadata": keypair.id.metadata,
            "created_at": chrono::Utc::now().to_rfc3339(),
        })
    };
    
    let json_str = serde_json::to_string_pretty(&json_value)
        .map_err(|e| format!("Erro ao serializar JSON: {}", e))?;
        
    fs::write(file_path, json_str)
        .map_err(|e| format!("Erro ao salvar arquivo: {}", e))?;
        
    Ok(())
}