//! CLI para o sistema LogLine
//!
//! Este módulo fornece uma interface de linha de comando
//! para interagir com o sistema LogLine.

mod identity_commands;
mod timeline_commands;
mod spans_commands;
mod federation_commands;

use clap::{ArgMatches, Command};
pub use identity_commands::{id_commands, run_id_command};
pub use timeline_commands::{timeline_commands, run_timeline_command};
pub use spans_commands::{spans_commands, run_spans_command};
pub use federation_commands::{federation_commands, run_federation_command};

/// Constrói a interface de linha de comando principal
pub fn build_cli() -> Command<'static> {
    Command::new("logline")
        .version(env!("CARGO_PKG_VERSION"))
        .author("LogLine Team")
        .about("Sistema de registro distribuído para instituições computacionais")
        .subcommand(id_commands())
        .subcommand(timeline_commands())
        .subcommand(spans_commands())
        .subcommand(federation_commands())
}

/// Processa os comandos da linha de comando
pub fn process_commands(matches: ArgMatches) -> Result<(), String> {
    match matches.subcommand() {
        Some(("id", id_matches)) => {
            run_id_command(id_matches)
        },
        Some(("timeline", timeline_matches)) => {
            run_timeline_command(timeline_matches)
        },
        Some(("spans", spans_matches)) => {
            run_spans_command(spans_matches)
        },
        Some(("federation", federation_matches)) => {
            run_federation_command(federation_matches)
        },
        _ => {
            // Nenhum comando conhecido foi passado, mostra ajuda
            println!("Uso: logline [COMMAND] [OPTIONS]");
            println!("Execute 'logline --help' para mais informações.");
            Ok(())
        }
    }
}