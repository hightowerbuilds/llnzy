//! Conversions between UTF-16 indices (the AppKit / `NSTextInputClient`
//! convention) and Rust character indices (the `StackerSession` /
//! `StackerSelection` convention).
//!
//! macOS reports text positions to applications as UTF-16 code unit offsets.
//! `StackerSelection` and `Buffer` work in Unicode scalar (`char`) offsets.
//! These helpers do the per-call translation. They run on input-protocol
//! callbacks, which are infrequent and bounded in size, so the linear-scan
//! cost is fine.

pub fn utf16_index_to_char_index(text: &str, utf16_index: usize) -> usize {
    let mut units = 0;
    for (char_index, ch) in text.chars().enumerate() {
        if units >= utf16_index {
            return char_index;
        }
        units += ch.len_utf16();
        if units > utf16_index {
            return char_index + 1;
        }
    }
    text.chars().count()
}

pub fn char_index_to_utf16_index(text: &str, char_index: usize) -> usize {
    text.chars().take(char_index).map(|ch| ch.len_utf16()).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_round_trips_one_to_one() {
        let s = "hello";
        for i in 0..=s.chars().count() {
            assert_eq!(
                utf16_index_to_char_index(s, char_index_to_utf16_index(s, i)),
                i
            );
        }
    }

    #[test]
    fn surrogate_pair_takes_two_utf16_units() {
        // U+1F600 (😀) is one Rust char but two UTF-16 code units.
        let s = "ab😀c";
        assert_eq!(char_index_to_utf16_index(s, 0), 0);
        assert_eq!(char_index_to_utf16_index(s, 1), 1);
        assert_eq!(char_index_to_utf16_index(s, 2), 2);
        assert_eq!(char_index_to_utf16_index(s, 3), 4);
        assert_eq!(char_index_to_utf16_index(s, 4), 5);

        assert_eq!(utf16_index_to_char_index(s, 0), 0);
        assert_eq!(utf16_index_to_char_index(s, 1), 1);
        assert_eq!(utf16_index_to_char_index(s, 2), 2);
        assert_eq!(utf16_index_to_char_index(s, 3), 3);
        assert_eq!(utf16_index_to_char_index(s, 4), 3);
        assert_eq!(utf16_index_to_char_index(s, 5), 4);
    }

    #[test]
    fn out_of_bounds_clamps_to_end() {
        let s = "abc";
        assert_eq!(utf16_index_to_char_index(s, 100), 3);
        assert_eq!(char_index_to_utf16_index(s, 100), 3);
    }

    #[test]
    fn unicode_corpus_round_trips_valid_character_boundaries() {
        for text in [
            "",
            "plain ascii",
            "aé文z",
            "a😀b",
            "line one\n𝄞 music\nemoji 😀",
            "क्ष and flags 🇺🇸",
        ] {
            for char_index in 0..=text.chars().count() {
                let utf16_index = char_index_to_utf16_index(text, char_index);
                assert_eq!(
                    utf16_index_to_char_index(text, utf16_index),
                    char_index,
                    "valid UTF-16 boundary should round-trip for {text:?}"
                );
            }
        }
    }

    #[test]
    fn arbitrary_utf16_offsets_map_to_neighboring_character_boundaries() {
        for text in ["a😀b", "é\n𝄞x", "क्ष🇺🇸z"] {
            let total_units = char_index_to_utf16_index(text, text.chars().count());

            for utf16_index in 0..=total_units + 2 {
                let char_index = utf16_index_to_char_index(text, utf16_index);
                let clamped_utf16 = utf16_index.min(total_units);
                let mapped_utf16 = char_index_to_utf16_index(text, char_index);

                assert!(
                    mapped_utf16 >= clamped_utf16,
                    "mapped boundary should not precede requested UTF-16 offset"
                );
                if char_index > 0 {
                    let previous_utf16 = char_index_to_utf16_index(text, char_index - 1);
                    assert!(
                        previous_utf16 <= clamped_utf16,
                        "previous boundary should not exceed requested UTF-16 offset"
                    );
                }
            }
        }
    }
}
