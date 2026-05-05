use regex::Regex;

/// Detect URLs in a line of terminal text.
/// Returns a list of (start_col, end_col, url_string) tuples.
pub fn detect_urls(line: &str) -> Vec<(usize, usize, String)> {
    static URL_RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let re = URL_RE.get_or_init(|| {
        Regex::new(r#"(?:https?://|file://)[^\s<>"'`\)\]\}]+"#).expect("URL regex")
    });
    re.find_iter(line)
        .map(|m| (m.start(), m.end(), m.as_str().to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_urls_finds_https() {
        let urls = detect_urls("Visit https://example.com for details");
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0].2, "https://example.com");
    }

    #[test]
    fn detect_urls_finds_http() {
        let urls = detect_urls("Link: http://foo.bar/baz?x=1&y=2");
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0].2, "http://foo.bar/baz?x=1&y=2");
    }

    #[test]
    fn detect_urls_finds_file() {
        let urls = detect_urls("file:///tmp/test.txt");
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0].2, "file:///tmp/test.txt");
    }

    #[test]
    fn detect_urls_multiple() {
        let urls = detect_urls("See https://a.com and https://b.com");
        assert_eq!(urls.len(), 2);
        assert_eq!(urls[0].2, "https://a.com");
        assert_eq!(urls[1].2, "https://b.com");
    }

    #[test]
    fn detect_urls_none() {
        let urls = detect_urls("no links here at all");
        assert!(urls.is_empty());
    }

    #[test]
    fn detect_urls_returns_correct_columns() {
        let line = "  https://x.co  ";
        let urls = detect_urls(line);
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0].0, 2);
        assert_eq!(urls[0].1, 14);
    }
}
