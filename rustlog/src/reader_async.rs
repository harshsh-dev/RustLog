use std::io::SeekFrom;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::Result;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, AsyncSeekExt, BufReader};
use tokio::sync::mpsc::Sender;
use tokio::time::{sleep, Duration};

/// Async `tail -f`: seeks to **EOF** first, then reads new data with adaptive idle backoff.
///
/// `line_filter`: `None` sends every line; `Some(k)` only lines containing `k` (fewer allocations
/// and less channel traffic than filtering in the consumer).
pub async fn tail_file_async<P: AsRef<Path>>(
    file_path: P,
    tx: Sender<String>,
    running: Arc<AtomicBool>,
    line_filter: Option<&str>,
) -> Result<()> {
    let mut file = File::open(file_path).await?;
    file.seek(SeekFrom::End(0)).await?;

    let mut reader = BufReader::new(file);
    let mut buffer = String::new();
    let mut idle_ms: u64 = 8;
    const MAX_IDLE_MS: u64 = 512;

    loop {
        if !running.load(Ordering::Relaxed) {
            break;
        }

        buffer.clear();
        let bytes_read = reader.read_line(&mut buffer).await?;

        if bytes_read > 0 {
            idle_ms = 8;
            let trimmed_len = buffer.trim_end().len();
            if trimmed_len == 0 {
                continue;
            }
            if let Some(kw) = line_filter {
                if !buffer[..trimmed_len].contains(kw) {
                    continue;
                }
            }
            buffer.truncate(trimmed_len);
            match tx.send(buffer).await {
                Ok(()) => buffer = String::new(),
                Err(e) => {
                    tracing::error!(error = %e, "Tail sender closed");
                    break;
                }
            }
        } else {
            sleep(Duration::from_millis(idle_ms)).await;
            idle_ms = (idle_ms.saturating_mul(2)).min(MAX_IDLE_MS);
        }
    }

    Ok(())
}
