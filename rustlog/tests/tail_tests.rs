use std::{fs::OpenOptions, io::Write, sync::{Arc, atomic::AtomicBool}, thread, time::Duration};
use std::sync::mpsc;
use rustlog::reader::tail_file;

#[test]
fn tail_reads_new_lines() {
    let path = "test_tail.log";

    // Create and write initial content
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)
        .unwrap();

    writeln!(file, "INFO: starting").unwrap();

    let (tx, rx) = mpsc::channel();
    let running = Arc::new(AtomicBool::new(true));

    let running_clone = running.clone();
    let path_clone = path.to_string();

    // Spawn tail thread
    thread::spawn(move || {
        tail_file(path_clone, tx, running_clone, None).unwrap();
    });

    // Append new line
    thread::sleep(Duration::from_millis(500));
    let mut file = OpenOptions::new()
        .append(true)
        .open(path)
        .unwrap();
    writeln!(file, "ERROR: new crash").unwrap();

    // Wait and receive
    thread::sleep(Duration::from_millis(1000));
    running.store(false, std::sync::atomic::Ordering::SeqCst);

    let output: Vec<_> = rx.try_iter().collect();
    assert!(output.iter().any(|line| line.contains("ERROR: new crash")));
}
