use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use anyhow::Result;

use crate::matcher::LineMatcher;

/// Streams a log file with **O(1)** memory: one line buffer is reused for the whole file.
/// Invokes `on_match` for each line where `matcher` matches (trimmed line, no trailing newline).
pub fn for_each_matching_line<P, F>(file_path: P, matcher: &LineMatcher, mut on_match: F) -> Result<()>
where
    P: AsRef<Path>,
    F: FnMut(&str),
{
    let file = File::open(file_path.as_ref())?;
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    loop {
        line.clear();
        let bytes = reader.read_line(&mut line)?;
        if bytes == 0 {
            break;
        }
        let trimmed = line.trim_end();
        if matcher.matches_line(trimmed) {
            on_match(trimmed);
        }
    }
    Ok(())
}

/// Blocking `tail -f` style reader: starts at **end of file** and forwards new complete lines.
///
/// `line_filter`: `None` forwards every line; `Some(m)` only lines matched by `m`.
pub fn tail_file<P: AsRef<Path>>(
    file_path: P,
    tx: Sender<String>,
    running: Arc<AtomicBool>,
    line_filter: Option<Arc<LineMatcher>>,
) -> Result<()> {
    let mut file = File::open(file_path)?;
    file.seek(SeekFrom::End(0))?;

    let mut reader = BufReader::new(file);
    let mut line = String::new();
    let mut idle_ms: u64 = 8;
    const MAX_IDLE_MS: u64 = 512;

    while running.load(Ordering::Relaxed) {
        line.clear();
        let bytes_read = reader.read_line(&mut line)?;

        if bytes_read > 0 {
            idle_ms = 8;
            let trimmed_len = line.trim_end().len();
            if trimmed_len == 0 {
                continue;
            }
            if let Some(m) = &line_filter {
                if !m.matches_line(&line[..trimmed_len]) {
                    continue;
                }
            }
            line.truncate(trimmed_len);
            tx.send(std::mem::take(&mut line))?;
        } else {
            thread::sleep(Duration::from_millis(idle_ms));
            idle_ms = (idle_ms.saturating_mul(2)).min(MAX_IDLE_MS);
        }
    }

    Ok(())
}
