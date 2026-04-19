pub mod args;
pub mod filter;
pub mod reader;
pub mod reader_async;

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::Result;
use tokio::signal;
use tokio::sync::mpsc;
use tracing_subscriber::{fmt, EnvFilter};

fn init_tracing() {
    let subscriber = fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .with_level(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        .compact()
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
}

/// CLI entry: filter a file once, or tail with graceful Ctrl+C shutdown.
pub async fn run() -> Result<()> {
    init_tracing();
    let args = args::parse_args();
    tracing::info!(?args, "Starting RustLog");

    if args.tail {
        tracing::info!(
            file = %Path::new(&args.file_path).display(),
            "Tail mode activated"
        );
        let (tx, mut rx) = mpsc::channel(64);
        let file_path = args.file_path.clone();
        let keyword = args.keyword.clone();

        let running = Arc::new(AtomicBool::new(true));
        let r = running.clone();

        tokio::spawn(async move {
            if signal::ctrl_c().await.is_err() {
                tracing::warn!("Ctrl+C handler unavailable");
            } else {
                tracing::info!("Shutting down gracefully...");
            }
            r.store(false, Ordering::Relaxed);
        });

        let r = running.clone();
        tokio::spawn(async move {
            if let Err(e) = reader_async::tail_file_async(
                file_path,
                tx,
                r,
                Some(keyword.as_str()),
            )
            .await
            {
                tracing::error!(error = %e, "Tailing failed");
            }
        });

        while let Some(line) = rx.recv().await {
            tracing::info!(target: "filtered", %line);
        }
    } else {
        tracing::info!(
            file = %Path::new(&args.file_path).display(),
            "Non-tail mode: streaming file"
        );
        if let Err(e) =
            reader::for_each_matching_line(&args.file_path, &args.keyword, |line| {
                tracing::info!(target: "filtered", line = %line);
            })
        {
            tracing::error!(error = %e, "Failed to read file");
        }
    }

    Ok(())
}
