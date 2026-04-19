/// Returns true when `line` should be emitted for the given `keyword`.
#[inline]
pub fn line_matches(line: &str, keyword: &str) -> bool {
    line.contains(keyword)
}

/// Filters lines from a vector of strings based on a keyword.
/// Prefer streaming APIs such as [`crate::reader::for_each_matching_line`] for large files.
pub fn filter_lines(lines: Vec<String>, keyword: &str) -> Vec<String> {
    lines
        .into_iter()
        .filter(|line| line_matches(line, keyword))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_lines() {
        let lines = vec![
            "INFO: Start".to_string(),
            "ERROR: Fail".to_string(),
            "WARN: Warn".to_string(),
        ];
        let result = filter_lines(lines, "ERROR");
        assert_eq!(result, vec!["ERROR: Fail"]);
    }

    #[test]
    fn test_filter_no_match() {
        let lines = vec![
            "INFO: Start".to_string(),
            "WARN: Warn".to_string(),
        ];
        let result = filter_lines(lines, "FATAL");
        assert!(result.is_empty());
    }
}
