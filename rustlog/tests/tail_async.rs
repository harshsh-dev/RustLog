use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::time::timeout;

use rustlog::reader_async::tail_file_async; // adjust path as needed
use std::sync::atomic::{AtomicBool, Ordering};

#[tokio::test]
async fn integration_tail_logs_correctly() {
    // Setup: temp file
    let temp_file = tempfile::NamedTempFile::new().unwrap();
    let path = temp_file.path().to_path_buf();

    // Setup: channel & shutdown flag
    let (tx, mut rx) = mpsc::channel(10);
    let running = Arc::new(AtomicBool::new(true));

    // Start tailing the file
    let tail_handle = {
        let path = path.clone();
        let running = running.clone();
        tokio::spawn(async move {
            tail_file_async(path, tx, running, None).await.unwrap();
        })
    };

    // Let the tail task open the (empty) file and seek to EOF before we append; otherwise we
    // would attach after existing bytes and miss them — same semantics as `tail -f`.
    tokio::time::sleep(Duration::from_millis(75)).await;

    // Write two lines to the file
    let mut file = OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap();

    writeln!(file, "INFO: system online").unwrap();
    writeln!(file, "ERROR: something went wrong").unwrap();
    file.flush().unwrap();

    // Give time for the watcher to detect changes
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Wait until we find the expected log
    let mut matched = false;
    let timeout_duration = Duration::from_secs(2);

    while let Ok(Some(line)) = timeout(timeout_duration, rx.recv()).await {
        println!("Received line: {line:?}");
        if line.contains("something went wrong") {
            matched = true;
            break;
        }
    }

    // Cleanup: signal shutdown
    running.store(false, Ordering::SeqCst);
    tail_handle.abort(); // stop the task

    assert!(matched, "Expected log line was not found");
}
