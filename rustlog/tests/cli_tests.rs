use assert_cmd::Command;
use std::fs::write;
use tempfile::NamedTempFile;

#[test]
fn shows_filtered_log_output() {
    let file = NamedTempFile::new().expect("Failed to create temp file");
    let log_content = "INFO: start\nERROR: something went wrong\nWARN: disk almost full";
    write(file.path(), log_content).expect("Failed to write to temp file");

    let mut cmd = Command::cargo_bin("rustlog").unwrap();
    let assert = cmd
        .arg(file.path())
        .arg("ERROR")
        .env("RUST_LOG", "info")
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout);
    let err = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        out.contains("ERROR: something went wrong")
            || err.contains("ERROR: something went wrong")
            || out.contains("matched log line")
            || err.contains("matched log line"),
        "stdout={out:?} stderr={err:?}"
    );
}

#[test]
fn config_file_filters_same_as_keyword_mode() {
    let log = NamedTempFile::new().unwrap();
    write(
        log.path(),
        "INFO: a\nERROR: cfg hit\nWARN: b\n",
    )
    .unwrap();

    let cfg = NamedTempFile::new().unwrap();
    write(
        cfg.path(),
        format!(
            r#"
[source]
path = "{}"

[filters]
patterns = ["ERROR"]
mode = "any"
"#,
            log.path().display()
        ),
    )
    .unwrap();

    let assert = Command::cargo_bin("rustlog")
        .unwrap()
        .arg("-C")
        .arg(cfg.path())
        .env("RUST_LOG", "info")
        .assert()
        .success();
    let err = String::from_utf8_lossy(&assert.get_output().stderr);
    let out = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        err.contains("cfg hit")
            || out.contains("cfg hit")
            || err.contains("matched log line")
            || out.contains("matched log line"),
        "stderr={err:?} stdout={out:?}"
    );
}
