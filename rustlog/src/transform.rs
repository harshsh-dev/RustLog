//! Built-in transform plugins (TOML-configurable pipeline).

use std::sync::Arc;

use anyhow::{Context, Result};
use regex::Regex;
use serde::Deserialize;

/// One step in `rustlog.toml` `[[transforms]]` tables.
#[derive(Debug, Deserialize, Clone)]
pub struct TransformSpec {
    pub name: String,
    #[serde(default)]
    pub arg: Option<String>,
    #[serde(default)]
    pub pattern: Option<String>,
    #[serde(default)]
    pub replacement: Option<String>,
}

pub trait Transform: Send + Sync {
    /// Returns `None` to drop the line; `Some(s)` to pass to the next step.
    fn apply(&self, line: String) -> Option<String>;
}

pub type TransformArc = Arc<dyn Transform>;

pub fn build_pipeline(specs: &[TransformSpec]) -> Result<Vec<TransformArc>> {
    let mut out = Vec::with_capacity(specs.len());
    for s in specs {
        let t: TransformArc = match s.name.as_str() {
            "trim" => Arc::new(Trim),
            "lowercase" => Arc::new(Lowercase),
            "uppercase" => Arc::new(Uppercase),
            "strip_prefix" => {
                let p = s
                    .arg
                    .clone()
                    .context("strip_prefix requires string `arg`")?;
                Arc::new(StripPrefix(p))
            }
            "prepend" => {
                let p = s
                    .arg
                    .clone()
                    .context("prepend requires string `arg`")?;
                Arc::new(Prepend(p))
            }
            "append" => {
                let p = s
                    .arg
                    .clone()
                    .context("append requires string `arg`")?;
                Arc::new(Append(p))
            }
            "regex_replace" => {
                let pat = s
                    .pattern
                    .clone()
                    .context("regex_replace requires `pattern`")?;
                let rep = s
                    .replacement
                    .clone()
                    .context("regex_replace requires `replacement`")?;
                Arc::new(RegexReplace::new(&pat, &rep)?)
            }
            "drop_if_matches" => {
                let pat = s
                    .pattern
                    .clone()
                    .context("drop_if_matches requires `pattern` regex")?;
                Arc::new(DropIfMatches::new(&pat)?)
            }
            "drop_if_empty" => Arc::new(DropIfEmpty),
            other => anyhow::bail!("unknown transform plugin: {other:?}"),
        };
        out.push(t);
    }
    Ok(out)
}

/// Apply each transform in order. `None` means the line must not be emitted.
pub fn apply_pipeline(line: &str, chain: &[TransformArc]) -> Option<String> {
    if chain.is_empty() {
        return Some(line.to_string());
    }
    let mut cur = line.to_string();
    for step in chain {
        cur = step.apply(cur)?;
    }
    Some(cur)
}

struct Trim;
impl Transform for Trim {
    fn apply(&self, line: String) -> Option<String> {
        Some(line.trim().to_string())
    }
}

struct Lowercase;
impl Transform for Lowercase {
    fn apply(&self, line: String) -> Option<String> {
        Some(line.to_lowercase())
    }
}

struct Uppercase;
impl Transform for Uppercase {
    fn apply(&self, line: String) -> Option<String> {
        Some(line.to_uppercase())
    }
}

struct StripPrefix(String);
impl Transform for StripPrefix {
    fn apply(&self, mut line: String) -> Option<String> {
        if line.starts_with(&self.0) {
            line.drain(..self.0.len());
        }
        Some(line)
    }
}

struct Prepend(String);
impl Transform for Prepend {
    fn apply(&self, line: String) -> Option<String> {
        Some(format!("{}{}", self.0, line))
    }
}

struct Append(String);
impl Transform for Append {
    fn apply(&self, line: String) -> Option<String> {
        Some(format!("{}{}", line, self.0))
    }
}

struct RegexReplace {
    re: Regex,
    replacement: String,
}

impl RegexReplace {
    fn new(pattern: &str, replacement: &str) -> Result<Self> {
        Ok(Self {
            re: Regex::new(pattern).with_context(|| format!("invalid regex: {pattern:?}"))?,
            replacement: replacement.to_string(),
        })
    }
}

impl Transform for RegexReplace {
    fn apply(&self, line: String) -> Option<String> {
        Some(self.re.replace_all(&line, &self.replacement).to_string())
    }
}

struct DropIfMatches(Regex);

impl DropIfMatches {
    fn new(pattern: &str) -> Result<Self> {
        Ok(Self(
            Regex::new(pattern).with_context(|| format!("invalid regex: {pattern:?}"))?,
        ))
    }
}

impl Transform for DropIfMatches {
    fn apply(&self, line: String) -> Option<String> {
        if self.0.is_match(&line) {
            None
        } else {
            Some(line)
        }
    }
}

struct DropIfEmpty;

impl Transform for DropIfEmpty {
    fn apply(&self, line: String) -> Option<String> {
        if line.trim().is_empty() {
            None
        } else {
            Some(line)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trim_and_lowercase_chain() {
        let chain: Vec<TransformArc> = vec![
            Arc::new(Trim),
            Arc::new(Lowercase),
        ];
        let out = apply_pipeline("  HELLO  ", &chain).unwrap();
        assert_eq!(out, "hello");
    }

    #[test]
    fn strip_prefix_works() {
        let chain: Vec<TransformArc> = vec![Arc::new(StripPrefix("LOG:".into()))];
        assert_eq!(
            apply_pipeline("LOG:hello", &chain).unwrap(),
            "hello"
        );
    }

    #[test]
    fn drop_if_matches() {
        let chain: Vec<TransformArc> =
            vec![Arc::new(DropIfMatches::new("^SKIP").unwrap())];
        assert!(apply_pipeline("SKIP this", &chain).is_none());
        assert_eq!(
            apply_pipeline("KEEP this", &chain).unwrap(),
            "KEEP this"
        );
    }

    #[test]
    fn drop_if_empty_after_trim() {
        let chain: Vec<TransformArc> = vec![Arc::new(Trim), Arc::new(DropIfEmpty)];
        assert!(apply_pipeline("   \n", &chain).is_none());
        assert_eq!(apply_pipeline(" x ", &chain).unwrap(), "x");
    }

    #[test]
    fn build_unknown_plugin_errors() {
        let specs = vec![TransformSpec {
            name: "not-a-real-plugin".into(),
            arg: None,
            pattern: None,
            replacement: None,
        }];
        assert!(build_pipeline(&specs).is_err());
    }

    #[test]
    fn build_strip_prefix_missing_arg_errors() {
        let specs = vec![TransformSpec {
            name: "strip_prefix".into(),
            arg: None,
            pattern: None,
            replacement: None,
        }];
        assert!(build_pipeline(&specs).is_err());
    }

    #[test]
    fn regex_replace_invalid_pattern_errors() {
        let specs = vec![TransformSpec {
            name: "regex_replace".into(),
            arg: None,
            pattern: Some("(".into()),
            replacement: Some("".into()),
        }];
        assert!(build_pipeline(&specs).is_err());
    }

    #[test]
    fn build_pipeline_roundtrip() {
        let specs = vec![
            TransformSpec {
                name: "trim".into(),
                arg: None,
                pattern: None,
                replacement: None,
            },
            TransformSpec {
                name: "prepend".into(),
                arg: Some("P:".into()),
                pattern: None,
                replacement: None,
            },
        ];
        let chain = build_pipeline(&specs).unwrap();
        assert_eq!(apply_pipeline("  hi ", &chain).unwrap(), "P:hi");
    }
}
