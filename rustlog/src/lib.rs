pub mod args;
pub mod config;
pub mod filter;
pub mod matcher;
pub mod reader;
pub mod reader_async;
pub mod sink;
pub mod transform;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use transform::TransformArc;

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

    let resolved = config::ResolvedConfig::resolve(&args)?;
    tracing::info!(
        file = %resolved.file_path.display(),
        tail = args.tail,
        "Resolved configuration"
    );

    let matcher = resolved.matcher.arc();
    let pipeline: Arc<Vec<TransformArc>> =
        Arc::new(transform::build_pipeline(&resolved.transforms)?);
    let hub = sink::SinkHub::new(resolved.stdout, resolved.output_file.clone()).await?;

    let (tx, mut rx) = mpsc::channel(64);
    let file_path = resolved.file_path.clone();
    let running = Arc::new(AtomicBool::new(true));

    // Only install Ctrl+C for follow mode. In some non-interactive environments `ctrl_c()`
    // can resolve immediately; that would clear `running` before a one-shot file read starts.
    if args.tail {
        let r = running.clone();
        tokio::spawn(async move {
            match signal::ctrl_c().await {
                Ok(()) => {
                    tracing::info!("Shutting down gracefully...");
                    r.store(false, Ordering::Relaxed);
                }
                Err(_) => tracing::warn!(
                    "Ctrl+C handler unavailable; tail task may run until process is killed"
                ),
            }
        });
    }

    let r = running.clone();
    let tail_flag = args.tail;
    let m = matcher.clone();
    tokio::spawn(async move {
        let res = if tail_flag {
            reader_async::tail_file_async(file_path, tx, r, Some(m)).await
        } else {
            reader_async::stream_file_lines_once(file_path, tx, r, Some(m)).await
        };
        if let Err(e) = res {
            tracing::error!(error = %e, "Reader task failed");
        }
    });

    while let Some(line) = rx.recv().await {
        let Some(out) = transform::apply_pipeline(&line, pipeline.as_ref()) else {
            continue;
        };
        if let Err(e) = hub.emit(&out).await {
            tracing::error!(error = %e, "sink emit failed");
            return Err(e);
        }
    }

    Ok(())
}
