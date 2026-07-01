//! Shared prompt-packet validation helpers for measured agent compositions.
//!
//! Port of `runner/prompt_packet.py`. Lengths and character classes follow
//! Python semantics: `len`/slicing count Unicode **code points** (not bytes),
//! `str.strip`/`isspace` use Unicode whitespace, and `isalpha` uses the Unicode
//! Alphabetic property.

/// Longest run of identical consecutive characters (code points).
fn longest_run(chars: &[char]) -> usize {
    let mut longest = 0;
    let mut current = 0;
    let mut previous: Option<char> = None;
    for &ch in chars {
        if Some(ch) == previous {
            current += 1;
        } else {
            previous = Some(ch);
            current = 1;
        }
        longest = longest.max(current);
    }
    longest
}

/// Cheap syntactic guardrail against degenerate optimizer output.
///
/// This is not a semantic quality judge; the arena still measures that. It only
/// rejects packets that are too thin, absurdly large, or visibly corrupted, so
/// a bad optimizer call cannot poison the seed or mutation pool with text like
/// one repeated punctuation character.
pub fn is_sane_prompt_packet(text: &str) -> bool {
    // The Python guard `isinstance(text, str)` is subsumed by the `&str` type.
    let stripped: Vec<char> = text.trim().chars().collect();
    let len = stripped.len();
    if !(20..=4000).contains(&len) {
        return false;
    }
    if longest_run(&stripped) > 24 {
        return false;
    }
    let visible: Vec<char> = stripped
        .iter()
        .copied()
        .filter(|c| !c.is_whitespace())
        .collect();
    if visible.len() >= 120 {
        let alpha = visible.iter().filter(|c| c.is_alphabetic()).count();
        let alpha_ratio = alpha as f64 / visible.len() as f64;
        if alpha_ratio < 0.25 {
            return false;
        }
        let sample = &visible[..visible.len().min(500)];
        let unique: std::collections::HashSet<char> = sample.iter().copied().collect();
        let unique_ratio = unique.len() as f64 / sample.len() as f64;
        if unique_ratio < 0.05 {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_too_short() {
        assert!(!is_sane_prompt_packet("too short"));
    }

    #[test]
    fn rejects_long_runs() {
        assert!(!is_sane_prompt_packet(&"a".repeat(30)));
    }

    #[test]
    fn rejects_oversized() {
        // 4010 code points, no long runs, all alpha -> fails only the size cap
        assert!(!is_sane_prompt_packet(&"abcdefghij".repeat(401)));
    }

    #[test]
    fn accepts_a_normal_packet() {
        let packet = "Review the diff for correctness and security issues; \
                      cite file and line for every finding you report.";
        assert!(is_sane_prompt_packet(packet));
    }

    #[test]
    fn rejects_low_alpha_ratio() {
        assert!(!is_sane_prompt_packet(&"1234567890!@#$%^&*()".repeat(10)));
    }

    #[test]
    fn rejects_low_unique_ratio() {
        // alpha ratio 1.0 but only two distinct characters across 200
        assert!(!is_sane_prompt_packet(&"ab".repeat(100)));
    }

    #[test]
    fn counts_code_points_not_bytes() {
        // 10 code points but 20 bytes: Python len() counts code points, so this
        // is below the >=20 floor. A bytes-based impl would wrongly accept it.
        assert_eq!("é".repeat(10).len(), 20); // bytes
        assert!(!is_sane_prompt_packet(&"é".repeat(10)));
    }
}
