mod backend;
mod convert;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use backend::Backend;
use clap::{Parser, Subcommand};
use spice_parser::{format_source, FormatOptions};
use tower_lsp::{LspService, Server};

#[derive(Parser, Debug)]
#[command(
    name = "spice-lsp",
    version,
    about = "Language server and formatter for SPICE netlists"
)]
struct Cli {
    /// Communicate over stdio (default; accepted for LSP client compatibility)
    #[arg(long, global = true)]
    stdio: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Format SPICE netlist files
    Format {
        /// Exit with status 1 if any file would change
        #[arg(long)]
        check: bool,
        /// Write formatted output back to each file
        #[arg(long)]
        write: bool,
        /// Files to format (stdin not supported)
        #[arg(required = true)]
        files: Vec<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    let Cli {
        command,
        stdio: _stdio,
    } = Cli::parse();
    // Transport is always stdio today; `_stdio` exists so clients may pass `--stdio`.
    match command {
        None => {
            run_lsp().await;
            ExitCode::SUCCESS
        }
        Some(Commands::Format {
            check,
            write,
            files,
        }) => match run_format(check, write, &files) {
            Ok(code) => code,
            Err(err) => {
                eprintln!("error: {err}");
                ExitCode::FAILURE
            }
        },
    }
}

async fn run_lsp() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}

fn run_format(check: bool, write: bool, files: &[PathBuf]) -> Result<ExitCode, String> {
    if check && write {
        return Err("use either --check or --write, not both".into());
    }

    let options = FormatOptions::default();
    let mut needs_format = false;

    for path in files {
        let original =
            std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let formatted = format_source(&original, &options);
        if formatted == original {
            continue;
        }
        needs_format = true;
        if write {
            std::fs::write(path, &formatted).map_err(|e| format!("{}: {e}", path.display()))?;
            eprintln!("wrote {}", display_path(path));
        } else if check {
            eprintln!("would reformat {}", display_path(path));
        } else {
            // Default: print formatted content for a single file; for multiple, write to stdout
            // with a clear separator only when needed.
            if files.len() == 1 {
                print!("{formatted}");
            } else {
                println!("--- {}", display_path(path));
                print!("{formatted}");
                if !formatted.ends_with('\n') {
                    println!();
                }
            }
        }
    }

    if check && needs_format {
        Ok(ExitCode::FAILURE)
    } else {
        Ok(ExitCode::SUCCESS)
    }
}

fn display_path(path: &Path) -> String {
    path.display().to_string()
}
