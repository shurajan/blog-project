use blog_server::{AppConfig, AppError, init_logging, run_app};
use clap::Parser;
use tokio_util::sync::CancellationToken;
use tracing::info;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long = "http_port", default_value_t = 3000)]
    http_port: u16,

    #[arg(long = "grpc_port", default_value_t = 50051)]
    grpc_port: u16,
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    init_logging()?;
    info!("starting blog server");

    let args = Args::parse();
    let config = AppConfig::from_env()?;
    let shutdown = CancellationToken::new();

    let signal_shutdown = shutdown.clone();
    tokio::spawn(async move {
        wait_for_signal().await;
        info!("shutdown signal received, stopping all servers");
        signal_shutdown.cancel();
    });

    run_app(config, args.http_port, args.grpc_port, shutdown).await?;

    info!("blog server shut down");
    Ok(())
}

async fn wait_for_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};

        let mut sigterm = signal(SignalKind::terminate()).expect("install SIGTERM handler");
        let mut sigint = signal(SignalKind::interrupt()).expect("install SIGINT handler");

        tokio::select! {
            _ = sigterm.recv() => info!("SIGTERM received"),
            _ = sigint.recv() => info!("SIGINT received"),
        }
    }

    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
        info!("ctrl-c received");
    }
}
