//! TOML configuration and merge with CLI flags.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::matcher::{LineMatcher, MatchMode};
use crate::transform::TransformSpec;

#[derive(Debug, Deserialize, Default, Clone)]
pub struct SourceSection {
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct FiltersSection {
    #[serde(default)]
    pub patterns: Vec<String>,
    #[serde(default)]
    pub use_regex: bool,
    #[serde(default)]
    pub mode: MatchMode,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OutputSection {
    /// When false, suppress tracing sink (still useful with `file` only).
    #[serde(default = "default_true")]
    pub stdout: bool,
    #[serde(default)]
    pub file: Option<String>,
}

fn default_true() -> bool {
    true
}

/// Optional Kafka producer (`[kafka]` in TOML). Requires `--features kafka` when enabled.
#[derive(Debug, Deserialize, Default, Clone)]
pub struct KafkaSection {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub brokers: Vec<String>,
    #[serde(default)]
    pub topic: String,
    #[serde(default)]
    pub client_id: Option<String>,
}

/// Optional Axum + WebSocket dashboard (`[web]` or `--web`).
#[derive(Debug, Deserialize, Default, Clone)]
pub struct WebSection {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub bind: Option<String>,
}

impl Default for OutputSection {
    fn default() -> Self {
        Self {
            stdout: true,
            file: None,
        }
    }
}

/// On-disk shape (extends with transforms / kafka / web in later modules).
#[derive(Debug, Deserialize, Default, Clone)]
pub struct FileConfig {
    #[serde(default)]
    pub source: SourceSection,
    #[serde(default)]
    pub filters: FiltersSection,
    #[serde(default)]
    pub output: OutputSection,
    #[serde(default)]
    pub transforms: Vec<TransformSpec>,
    #[serde(default)]
    pub kafka: KafkaSection,
    #[serde(default)]
    pub web: WebSection,
}

impl FileConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let text = std::fs::read_to_string(path.as_ref())
            .with_context(|| format!("read config {}", path.as_ref().display()))?;
        toml::from_str(&text).context("parse TOML config")
    }
}

/// Fully resolved paths and filter after merging CLI with optional file config.
#[derive(Debug, Clone)]
pub struct ResolvedConfig {
    pub file_path: PathBuf,
    pub matcher: LineMatcher,
    pub stdout: bool,
    pub output_file: Option<PathBuf>,
    pub transforms: Vec<TransformSpec>,
    pub kafka: KafkaSection,
    /// `Some("host:port")` when the dashboard should listen.
    pub web_bind: Option<String>,
}

impl ResolvedConfig {
    pub fn resolve(args: &crate::args::Args) -> Result<Self> {
        let file_cfg = if let Some(p) = &args.config {
            Some(FileConfig::load(p)?)
        } else {
            None
        };

        let file_path = args
            .file_path
            .clone()
            .or_else(|| file_cfg.as_ref()?.source.path.clone())
            .context("log file path: pass FILE positional or set [source].path in config")?;

        let filters = file_cfg
            .as_ref()
            .map(|c| c.filters.clone())
            .unwrap_or_default();

        let (patterns, use_regex, mode) = if let Some(kw) = args.keyword.clone() {
            (vec![kw], false, MatchMode::Any)
        } else if !filters.patterns.is_empty() {
            (
                filters.patterns.clone(),
                filters.use_regex,
                filters.mode,
            )
        } else if file_cfg.is_some() {
            (Vec::new(), false, MatchMode::Any)
        } else {
            anyhow::bail!(
                "filter: pass KEYWORD positional or set [filters].patterns in --config TOML"
            );
        };

        let matcher = LineMatcher::from_options(patterns, use_regex, mode)?;

        let output = file_cfg
            .as_ref()
            .map(|c| c.output.clone())
            .unwrap_or_default();

        let stdout = output.stdout;
        let output_file = args
            .out_file
            .clone()
            .or_else(|| output.file.as_ref().map(PathBuf::from));

        let transforms = file_cfg
            .as_ref()
            .map(|c| c.transforms.clone())
            .unwrap_or_default();

        let kafka = file_cfg
            .as_ref()
            .map(|c| c.kafka.clone())
            .unwrap_or_default();

        let web = file_cfg
            .as_ref()
            .map(|c| c.web.clone())
            .unwrap_or_default();

        let web_bind = args.web.clone().or_else(|| {
            if web.enabled {
                web.bind
                    .clone()
                    .or_else(|| Some("127.0.0.1:8080".to_string()))
            } else {
                None
            }
        });

        Ok(Self {
            file_path: PathBuf::from(file_path),
            matcher,
            stdout,
            output_file,
            transforms,
            kafka,
            web_bind,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::Args;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn write_config(content: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f.flush().unwrap();
        f
    }

    #[test]
    fn resolve_web_bind_from_config_when_enabled() {
        let cfg = write_config(
            r#"
[source]
path = "/a.log"

[filters]
patterns = ["x"]

[web]
enabled = true
bind = "127.0.0.1:19992"
"#,
        );
        let args = Args {
            config: Some(cfg.path().to_path_buf()),
            file_path: None,
            keyword: None,
            tail: false,
            out_file: None,
            web: None,
        };
        let r = ResolvedConfig::resolve(&args).unwrap();
        assert_eq!(r.web_bind.as_deref(), Some("127.0.0.1:19992"));
    }

    #[test]
    fn resolve_web_cli_overrides_config_bind() {
        let cfg = write_config(
            r#"
[source]
path = "/a.log"
[filters]
patterns = ["x"]
[web]
enabled = true
bind = "127.0.0.1:1"
"#,
        );
        let args = Args {
            config: Some(cfg.path().to_path_buf()),
            file_path: None,
            keyword: None,
            tail: false,
            out_file: None,
            web: Some("127.0.0.1:19993".into()),
        };
        let r = ResolvedConfig::resolve(&args).unwrap();
        assert_eq!(r.web_bind.as_deref(), Some("127.0.0.1:19993"));
    }

    #[test]
    fn load_config_with_transforms_array() {
        let f = write_config(
            r#"
[source]
path = "/tmp/a.log"

[filters]
patterns = ["E"]

[[transforms]]
name = "trim"

[[transforms]]
name = "prepend"
arg = ">> "
"#,
        );
        let c = FileConfig::load(f.path()).unwrap();
        assert_eq!(c.transforms.len(), 2);
        assert_eq!(c.transforms[0].name, "trim");
        assert_eq!(c.transforms[1].arg.as_deref(), Some(">> "));
    }

    #[test]
    fn load_minimal_config() {
        let f = write_config(
            r#"
[source]
path = "/tmp/x.log"

[filters]
patterns = ["ERR"]
"#,
        );
        let c = FileConfig::load(f.path()).unwrap();
        assert_eq!(c.source.path.as_deref(), Some("/tmp/x.log"));
        assert_eq!(c.filters.patterns, vec!["ERR"]);
    }

    #[test]
    fn resolve_cli_overrides_config_path() {
        let cfg = write_config(
            r#"
[source]
path = "/from/config.log"

[filters]
patterns = ["X"]
"#,
        );
        let args = Args {
            config: Some(cfg.path().to_path_buf()),
            file_path: Some("/from/cli.log".into()),
            keyword: None,
            tail: false,
            out_file: None,
            web: None,
        };
        let r = ResolvedConfig::resolve(&args).unwrap();
        assert_eq!(r.file_path, PathBuf::from("/from/cli.log"));
        assert!(r.matcher.matches_line("X"));
    }

    #[test]
    fn resolve_keyword_overrides_config_patterns() {
        let cfg = write_config(
            r#"
[source]
path = "/a.log"

[filters]
patterns = ["IGNORE"]
"#,
        );
        let args = Args {
            config: Some(cfg.path().to_path_buf()),
            file_path: None,
            keyword: Some("REAL".into()),
            tail: false,
            out_file: None,
            web: None,
        };
        let r = ResolvedConfig::resolve(&args).unwrap();
        assert!(r.matcher.matches_line("REAL hit"));
        assert!(!r.matcher.matches_line("IGNORE hit"));
    }

    #[test]
    fn resolve_legacy_positionals_without_config() {
        let args = Args {
            config: None,
            file_path: Some("f.log".into()),
            keyword: Some("k".into()),
            tail: false,
            out_file: None,
            web: None,
        };
        let r = ResolvedConfig::resolve(&args).unwrap();
        assert_eq!(r.file_path, PathBuf::from("f.log"));
        assert!(r.matcher.matches_line("kk"));
    }

    #[test]
    fn resolve_errors_without_path() {
        let args = Args {
            config: None,
            file_path: None,
            keyword: Some("k".into()),
            tail: false,
            out_file: None,
            web: None,
        };
        assert!(ResolvedConfig::resolve(&args).is_err());
    }

    #[test]
    fn resolve_match_all_when_config_has_no_patterns() {
        let cfg = write_config(
            r#"
[source]
path = "/a.log"
"#,
        );
        let args = Args {
            config: Some(cfg.path().to_path_buf()),
            file_path: None,
            keyword: None,
            tail: false,
            out_file: None,
            web: None,
        };
        let r = ResolvedConfig::resolve(&args).unwrap();
        assert!(r.matcher.matches_line("anything goes"));
    }

    #[test]
    fn output_section_default_stdout_enabled() {
        let o = OutputSection::default();
        assert!(o.stdout);
        assert!(o.file.is_none());
    }

    #[test]
    fn resolve_cli_out_file_overrides_config_output_file() {
        let dir = tempfile::tempdir().unwrap();
        let from_cfg = dir.path().join("cfg_out.log");
        let from_cli = dir.path().join("cli_out.log");
        let log = dir.path().join("app.log");
        std::fs::write(&log, b"x\n").unwrap();

        let cfg = write_config(&format!(
            r#"
[source]
path = "{}"

[filters]
patterns = ["x"]

[output]
file = "{}"
"#,
            log.display(),
            from_cfg.display()
        ));

        let args = Args {
            config: Some(cfg.path().to_path_buf()),
            file_path: None,
            keyword: None,
            tail: false,
            out_file: Some(from_cli.clone()),
            web: None,
        };
        let r = ResolvedConfig::resolve(&args).unwrap();
        assert_eq!(r.output_file.as_ref(), Some(&from_cli));
    }

    #[test]
    fn bad_toml_errors() {
        let f = write_config("not toml {{{");
        assert!(FileConfig::load(f.path()).is_err());
    }
}
