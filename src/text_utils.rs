use std::borrow::Cow;

pub fn contains_case_insensitive(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    if haystack.is_ascii() && needle.is_ascii() {
        return haystack
            .as_bytes()
            .windows(needle.len())
            .any(|window| window.eq_ignore_ascii_case(needle.as_bytes()));
    }
    haystack.to_lowercase().contains(&needle.to_lowercase())
}

pub fn fuzzy_match_case_insensitive_ascii(query_lower: &str, target: &str) -> bool {
    let mut target_chars = target.chars();
    for qc in query_lower.chars() {
        loop {
            match target_chars.next() {
                Some(tc) if tc.to_ascii_lowercase() == qc => break,
                Some(_) => continue,
                None => return false,
            }
        }
    }
    true
}

pub fn sort_by_cached_lowercase<T>(items: &mut [T], mut key: impl FnMut(&T) -> &str) {
    items.sort_by_cached_key(|item| key(item).to_lowercase());
}

pub fn normalize_crlf_to_lf(text: &str) -> Cow<'_, str> {
    if text.contains("\r\n") {
        Cow::Owned(text.replace("\r\n", "\n"))
    } else {
        Cow::Borrowed(text)
    }
}

pub fn normalize_crlf_and_cr_to_lf(text: &str) -> Cow<'_, str> {
    if text.contains('\r') {
        Cow::Owned(text.replace("\r\n", "\n").replace('\r', "\n"))
    } else {
        Cow::Borrowed(text)
    }
}

pub fn char_range_slice(text: &str, start: usize, end: usize) -> Option<&str> {
    if start >= end {
        return None;
    }

    let mut start_byte = None;
    let mut end_byte = text.len();
    for (char_idx, (byte_idx, _)) in text.char_indices().enumerate() {
        if char_idx == start {
            start_byte = Some(byte_idx);
        }
        if char_idx == end {
            end_byte = byte_idx;
            break;
        }
    }

    start_byte.map(|start_byte| &text[start_byte..end_byte])
}

pub fn truncate_chars(text: &str, max_chars: usize) -> Cow<'_, str> {
    let keep_chars = max_chars.saturating_sub(3);
    let mut cut_byte = (keep_chars == 0).then_some(0);
    let mut exceeded = false;

    for (char_idx, (byte_idx, _)) in text.char_indices().enumerate() {
        if char_idx == keep_chars {
            cut_byte = Some(byte_idx);
        }
        if char_idx == max_chars {
            exceeded = true;
            break;
        }
    }

    if !exceeded {
        return Cow::Borrowed(text);
    }

    let cut_byte = cut_byte.unwrap_or(text.len());
    let mut out = String::with_capacity(cut_byte + 3);
    out.push_str(&text[..cut_byte]);
    out.push_str("...");
    Cow::Owned(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_case_insensitive_handles_ascii_without_case() {
        assert!(contains_case_insensitive("Cargo Build", "build"));
        assert!(contains_case_insensitive("Cargo Build", "CARGO"));
        assert!(!contains_case_insensitive("Cargo Build", "test"));
    }

    #[test]
    fn fuzzy_match_case_insensitive_ascii_matches_ordered_chars() {
        assert!(fuzzy_match_case_insensitive_ascii("ct", "Close Tab"));
        assert!(!fuzzy_match_case_insensitive_ascii("tc", "Close Tab"));
    }

    #[test]
    fn normalize_crlf_to_lf_preserves_lone_carriage_returns() {
        assert_eq!(normalize_crlf_to_lf("a\r\nb\r c").as_ref(), "a\nb\r c");
    }

    #[test]
    fn normalize_crlf_and_cr_to_lf_converts_all_carriage_returns() {
        assert_eq!(
            normalize_crlf_and_cr_to_lf("a\r\nb\r c").as_ref(),
            "a\nb\n c"
        );
    }

    #[test]
    fn char_range_slice_uses_character_bounds_without_allocating() {
        assert_eq!(char_range_slice("aé文z", 1, 3), Some("é文"));
        assert_eq!(char_range_slice("aé文z", 2, 99), Some("文z"));
    }

    #[test]
    fn char_range_slice_rejects_empty_or_out_of_bounds_ranges() {
        assert_eq!(char_range_slice("abc", 1, 1), None);
        assert_eq!(char_range_slice("abc", 4, 5), None);
        assert_eq!(char_range_slice("", 0, 1), None);
    }

    #[test]
    fn truncate_chars_borrows_when_within_limit() {
        assert!(matches!(truncate_chars("short", 8), Cow::Borrowed("short")));
    }

    #[test]
    fn truncate_chars_matches_existing_ellipsis_behavior() {
        assert_eq!(truncate_chars("abcdefghij", 6).as_ref(), "abc...");
        assert_eq!(truncate_chars("abcdefghij", 2).as_ref(), "...");
        assert_eq!(truncate_chars("aé文def", 5).as_ref(), "aé...");
    }
}
