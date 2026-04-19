use std::sync::Arc;

use anyhow::{Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};

/// How multiple filter patterns combine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MatchMode {
    /// Line must match at least one pattern.
    #[default]
    Any,
    /// Line must match every pattern.
    All,
}

/// Compiled line filter: substring or full-regex patterns with `Any` / `All` semantics.
#[derive(Debug, Clone)]
pub struct LineMatcher {
    patterns: Vec<String>,
    regexes: Option<Vec<Regex>>,
    mode: MatchMode,
}

impl LineMatcher {
    /// Single substring match (`contains`), `Any` mode.
    pub fn keyword(keyword: impl Into<String>) -> Self {
        let s = keyword.into();
        Self {
            patterns: vec![s],
            regexes: None,
            mode: MatchMode::Any,
        }
    }

    /// Build matcher from TOML-style options. Empty `patterns` matches **every** line.
    pub fn from_options(
        patterns: Vec<String>,
        use_regex: bool,
        mode: MatchMode,
    ) -> Result<Self> {
        let regexes = if use_regex {
            let mut v = Vec::with_capacity(patterns.len());
            for p in &patterns {
                v.push(
                    Regex::new(p)
                        .with_context(|| format!("invalid filter regex: {p:?}"))?,
                );
            }
            Some(v)
        } else {
            None
        };
        Ok(Self {
            patterns,
            regexes,
            mode,
        })
    }

    /// Wrap in `Arc` for async tail / broadcast paths.
    pub fn arc(self) -> Arc<Self> {
        Arc::new(self)
    }

    #[inline]
    pub fn matches_line(&self, line: &str) -> bool {
        if self.patterns.is_empty() {
            return true;
        }
        if let Some(rxs) = &self.regexes {
            match self.mode {
                MatchMode::Any => rxs.iter().any(|r| r.is_match(line)),
                MatchMode::All => rxs.iter().all(|r| r.is_match(line)),
            }
        } else {
            match self.mode {
                MatchMode::Any => self.patterns.iter().any(|p| line.contains(p.as_str())),
                MatchMode::All => self
                    .patterns
                    .iter()
                    .all(|p| line.contains(p.as_str())),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_patterns_match_all() {
        let m = LineMatcher::from_options(vec![], false, MatchMode::Any).unwrap();
        assert!(m.matches_line("anything"));
        assert!(m.matches_line(""));
    }

    #[test]
    fn keyword_contains() {
        let m = LineMatcher::keyword("ERR");
        assert!(m.matches_line("pre ERROR post"));
        assert!(!m.matches_line("ok"));
    }

    #[test]
    fn substring_any_all() {
        let any = LineMatcher::from_options(vec!["a".into(), "b".into()], false, MatchMode::Any).unwrap();
        assert!(any.matches_line("ax"));
        assert!(any.matches_line("xb"));
        assert!(!any.matches_line("xy"));

        let all = LineMatcher::from_options(vec!["a".into(), "b".into()], false, MatchMode::All).unwrap();
        assert!(all.matches_line("ab"));
        assert!(!all.matches_line("a"));
    }

    #[test]
    fn regex_any_invalid_pattern_errors() {
        let err = LineMatcher::from_options(vec!["(".into()], true, MatchMode::Any);
        assert!(err.is_err());
    }

    #[test]
    fn regex_any_matches() {
        let m = LineMatcher::from_options(vec![r"\d{3}".into()], true, MatchMode::Any).unwrap();
        assert!(m.matches_line("code 500"));
        assert!(!m.matches_line("no digits"));
    }

    #[test]
    fn utf8_patterns() {
        let m = LineMatcher::from_options(vec!["日本語".into()], false, MatchMode::Any).unwrap();
        assert!(m.matches_line("log: 日本語 here"));
        assert!(!m.matches_line("ascii only"));
    }

    #[test]
    fn regex_all_mode() {
        let m = LineMatcher::from_options(
            vec![r"\bfoo\b".into(), r"\bbar\b".into()],
            true,
            MatchMode::All,
        )
        .unwrap();
        assert!(m.matches_line("foo and bar"));
        assert!(!m.matches_line("foo only"));
    }
}
