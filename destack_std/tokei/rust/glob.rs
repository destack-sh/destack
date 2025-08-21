/// Check if text matches glob pattern supporting '*' and '?' wildcards.
pub fn matches_glob(
    pattern: &[u8],
    mut pattern_idx: usize,
    text: &[u8],
    mut text_idx: usize,
) -> bool {
    let pattern_length = pattern.len();
    let text_length = text.len();
    let mut star_idx: Option<usize> = None;
    let mut match_idx: usize = 0;

    while text_idx < text_length {
        if pattern_idx < pattern_length
            && (pattern[pattern_idx] == b'?' || pattern[pattern_idx] == text[text_idx])
        {
            pattern_idx += 1;
            text_idx += 1;
        } else if pattern_idx < pattern_length && pattern[pattern_idx] == b'*' {
            star_idx = Some(pattern_idx);
            match_idx = text_idx;
            pattern_idx += 1;
        } else if let Some(si) = star_idx {
            pattern_idx = si + 1;
            match_idx += 1;
            text_idx = match_idx;
        } else {
            return false;
        }
    }

    while pattern_idx < pattern_length && pattern[pattern_idx] == b'*' {
        pattern_idx += 1;
    }
    pattern_idx == pattern_length
}
