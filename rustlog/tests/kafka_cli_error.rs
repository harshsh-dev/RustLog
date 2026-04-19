use assert_cmd::Command;
use std::fs::write;
use tempfile::NamedTempFile;

/// With default features, `[kafka].enabled = true` must fail fast with a clear message.
#[test]
fn kafka_enabled_without_feature_errors() {
    let log = NamedTempFile::new().unwrap();
    write(log.path(), "ERROR: x\n").unwrap();

    let cfg = NamedTempFile::new().unwrap();
    write(
        cfg.path(),
        format!(
            r#"
[source]
path = "{}"

[filters]
patterns = ["ERROR"]

[kafka]
enabled = true
brokers = ["127.0.0.1:9092"]
topic = "rustlog-test"
"#,
            log.path().display()
        ),
    )
    .unwrap();

    let assert = Command::cargo_bin("rustlog")
        .unwrap()
        .arg("-C")
        .arg(cfg.path())
        .assert()
        .failure();

    let err = String::from_utf8_lossy(&assert.get_output().stderr);
    let out = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        err.contains("kafka") || err.contains("Kafka") || out.contains("kafka") || out.contains("Kafka"),
        "stderr={err:?} stdout={out:?}"
    );
}
