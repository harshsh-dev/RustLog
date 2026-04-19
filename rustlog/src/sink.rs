//! Output sinks (stdout via tracing, optional log file).

use std::path::PathBuf;

use anyhow::{Context, Result};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

/// Tracing + optional append-only file output.
pub struct SinkHub {
    stdout: bool,
    file: Option<Mutex<tokio::fs::File>>,
}

impl SinkHub {
    pub async fn new(stdout: bool, output_file: Option<PathBuf>) -> Result<Self> {
        let file = if let Some(p) = output_file {
            let f = tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&p)
                .await
                .with_context(|| format!("open output file {}", p.display()))?;
            Some(Mutex::new(f))
        } else {
            None
        };
        Ok(Self { stdout, file })
    }

    pub async fn emit(&self, line: &str) -> Result<()> {
        if self.stdout {
            tracing::info!(line = %line, "matched log line");
        }
        if let Some(f) = &self.file {
            let mut g = f.lock().await;
            g.write_all(line.as_bytes()).await?;
            g.write_all(b"\n").await?;
            g.flush().await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tempfile::tempdir;

    #[tokio::test]
    async fn file_sink_appends_lines() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("out.log");
        let hub = SinkHub::new(false, Some(p.clone())).await.unwrap();
        hub.emit("one").await.unwrap();
        hub.emit("two").await.unwrap();
        let body = tokio::fs::read_to_string(&p).await.unwrap();
        assert_eq!(body, "one\ntwo\n");
    }

    #[tokio::test]
    async fn file_sink_utf8_roundtrip() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("u.log");
        let hub = SinkHub::new(false, Some(p.clone())).await.unwrap();
        hub.emit("日本語 🔧").await.unwrap();
        let body = tokio::fs::read_to_string(&p).await.unwrap();
        assert!(body.contains("日本語"));
    }

    #[tokio::test]
    async fn stdout_only_no_file() {
        let hub = SinkHub::new(true, None).await.unwrap();
        hub.emit("ok").await.unwrap();
    }

    #[tokio::test]
    async fn file_path_creates_intermediate_unsupported_but_parent_must_exist() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("nested").join("out.log");
        // create_dir for nested
        tokio::fs::create_dir_all(p.parent().unwrap()).await.unwrap();
        let hub = SinkHub::new(false, Some(p.clone())).await.unwrap();
        hub.emit("x").await.unwrap();
        assert!(Path::new(&p).exists());
    }
}
