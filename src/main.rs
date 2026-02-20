use clap::{Parser, Subcommand};
use std::path::PathBuf;
use wakeguard::{error::WakeguardError, logging, service, ui};

#[derive(Parser, Debug)]
#[command(name = "wakeguard")]
#[command(about = "Wakeguard Windows service skeleton", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    Run,
    Install {
        #[arg(long = "binary-path")]
        binary_path: Option<PathBuf>,
        #[arg(long = "no-ui", default_value_t = false)]
        no_ui: bool,
    },
    Uninstall,
    Once,
    Ui {
        #[arg(long = "onboarding", default_value_t = false)]
        onboarding: bool,
    },
}

fn main() -> Result<(), WakeguardError> {
    logging::init_logging()?;

    let cli = Cli::parse();
    match cli.command.unwrap_or(Command::Run) {
        Command::Run => service::run_service(),
        Command::Install { binary_path, no_ui } => service::install_service(binary_path, !no_ui),
        Command::Uninstall => service::uninstall_service(),
        Command::Once => service::worker::run_once(),
        Command::Ui { onboarding } => ui::launch_ui(onboarding),
    }
}
