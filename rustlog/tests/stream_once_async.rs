use std::sync::Arc;

use tokio::sync::mpsc;

use rustlog::matcher::LineMatcher;
use rustlog::reader_async::stream_file_lines_once;
use std::sync::atomic::AtomicBool;

#[tokio::test]
async fn stream_reads_from_start_not_tail() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("one.log");
    tokio::fs::write(&path, "SKIP\nKEEP\n")
        .await
        .unwrap();

    let (tx, mut rx) = mpsc::channel(8);
    let running = Arc::new(AtomicBool::new(true));
    let m = LineMatcher::keyword("KEEP").arc();

    let h = tokio::spawn(stream_file_lines_once(path, tx, running.clone(), Some(m)));
    let mut got = Vec::new();
    while let Some(l) = rx.recv().await {
        got.push(l);
    }
    h.await.unwrap().unwrap();
    assert_eq!(got, vec!["KEEP".to_string()]);
}

#[tokio::test]
async fn stream_empty_file_sends_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("empty.log");
    tokio::fs::write(&path, "").await.unwrap();

    let (tx, mut rx) = mpsc::channel(8);
    let running = Arc::new(AtomicBool::new(true));
    stream_file_lines_once(path, tx, running, None)
        .await
        .unwrap();
    assert!(rx.recv().await.is_none());
}

#[tokio::test]
async fn stream_finishes_when_no_lines_match() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("n.log");
    tokio::fs::write(&path, "alpha\nbeta\n").await.unwrap();

    let (tx, mut rx) = mpsc::channel(8);
    let running = Arc::new(AtomicBool::new(true));
    let m = LineMatcher::keyword("zzz").arc();
    stream_file_lines_once(path, tx, running, Some(m))
        .await
        .unwrap();
    assert!(rx.recv().await.is_none());
}
