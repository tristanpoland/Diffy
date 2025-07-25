use anyhow::Result;
use clap::Parser;
use diffy::{DiffyCore, TuiApp, start_server};
use std::path::PathBuf;
use tracing::Level;

#[derive(Parser)]
#[command(name = "diffy")]
#[command(about = "A modular CLI and web directory/file diff tool")]
#[command(version = "0.1.0")]
struct Cli {
    /// Left directory or file path
    #[arg(long, short)]
    left: PathBuf,

    /// Right directory or file path  
    #[arg(long, short)]
    right: PathBuf,

    /// Start web server instead of TUI
    #[arg(long)]
    web: bool,

    /// Port for web server (default: 3000)
    #[arg(long, default_value = "3000")]
    port: u16,

    /// Open browser automatically when using --web
    #[arg(long)]
    open: bool,

    /// Enable verbose logging
    #[arg(long, short)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Setup logging
    let log_level = if cli.verbose { Level::DEBUG } else { Level::INFO };
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .init();

    // Validate paths exist
    if !cli.left.exists() {
        eprintln!("Error: Left path '{}' does not exist", cli.left.display());
        std::process::exit(1);
    }
    
    if !cli.right.exists() {
        eprintln!("Error: Right path '{}' does not exist", cli.right.display());
        std::process::exit(1);
    }

    // Create core diff engine
    let core = DiffyCore::new(cli.left.clone(), cli.right.clone());

    if cli.web {
        // Open browser if requested
        if cli.open {
            let url = format!("http://127.0.0.1:{}", cli.port);
            if let Err(e) = webbrowser::open(&url) {
                eprintln!("Warning: Failed to open browser: {}", e);
                eprintln!("Please manually open: {}", url);
            }
        }

        // Start web server
        start_server(core, cli.port).await?;
    } else {
        // Start TUI
        let mut app = TuiApp::new(core);
        app.run()?;
    }

    Ok(())
}