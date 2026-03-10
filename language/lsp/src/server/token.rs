use destack_lsp_types as lsp;

/// A diff operation between semantic token streams.
#[derive(Debug)]
enum SemanticTokensDiffOp {
    /// A matching token in both streams.
    Equal,
    /// A token inserted from the next stream.
    Insert(lsp::SemanticToken),
    /// A token deleted from the previous stream.
    Delete,
}

/// Build a Myers diff between two semantic token payloads.
fn semantic_tokens_diff_ops(
    previous: &[lsp::SemanticToken],
    next: &[lsp::SemanticToken],
) -> Vec<SemanticTokensDiffOp> {
    let previous_len = previous.len() as isize;
    let next_len = next.len() as isize;
    if previous_len == 0 && next_len == 0 {
        return Vec::new();
    }

    let max = previous_len + next_len;
    let mut v = vec![0isize; (2 * max + 1) as usize];
    let mut trace = Vec::new();
    let mut end_d = 0usize;

    for d in 0..=max as usize {
        let d_isize = d as isize;
        let mut done = false;
        for k in (-d_isize..=d_isize).step_by(2) {
            let k_index = (k + max) as usize;
            let x = if k == -d_isize
                || (k != d_isize && v[(k - 1 + max) as usize] < v[(k + 1 + max) as usize])
            {
                v[(k + 1 + max) as usize]
            } else {
                v[(k - 1 + max) as usize] + 1
            };
            let mut x = x;
            let mut y = x - k;
            while x < previous_len && y < next_len && previous[x as usize] == next[y as usize] {
                x += 1;
                y += 1;
            }
            v[k_index] = x;
            if x >= previous_len && y >= next_len {
                done = true;
                break;
            }
        }
        trace.push(v.clone());
        if done {
            end_d = d;
            break;
        }
    }

    let mut ops = Vec::new();
    let mut x = previous_len;
    let mut y = next_len;

    for d in (1..=end_d).rev() {
        let d_isize = d as isize;
        let v_snapshot = &trace[d - 1];
        let k = x - y;
        let prev_k = if k == -d_isize
            || (k != d_isize
                && v_snapshot[(k - 1 + max) as usize] < v_snapshot[(k + 1 + max) as usize])
        {
            k + 1
        } else {
            k - 1
        };
        let prev_x = v_snapshot[(prev_k + max) as usize];
        let prev_y = prev_x - prev_k;

        while x > prev_x && y > prev_y {
            ops.push(SemanticTokensDiffOp::Equal);
            x -= 1;
            y -= 1;
        }

        if x == prev_x {
            let index = (y - 1) as usize;
            ops.push(SemanticTokensDiffOp::Insert(next[index]));
            y -= 1;
        } else {
            ops.push(SemanticTokensDiffOp::Delete);
            x -= 1;
        }
    }

    while x > 0 && y > 0 {
        ops.push(SemanticTokensDiffOp::Equal);
        x -= 1;
        y -= 1;
    }
    while x > 0 {
        ops.push(SemanticTokensDiffOp::Delete);
        x -= 1;
    }
    while y > 0 {
        let index = (y - 1) as usize;
        ops.push(SemanticTokensDiffOp::Insert(next[index]));
        y -= 1;
    }

    ops.reverse();
    ops
}

/// Push a semantic tokens edit if it carries changes.
fn push_semantic_tokens_edit(
    edits: &mut Vec<lsp::SemanticTokensEdit>,
    start: usize,
    delete_count: usize,
    data: &mut Vec<lsp::SemanticToken>,
) {
    if delete_count == 0 && data.is_empty() {
        return;
    }

    let data = if data.is_empty() {
        None
    } else {
        Some(std::mem::take(data))
    };

    edits.push(lsp::SemanticTokensEdit {
        start: start as u32,
        delete_count: delete_count as u32,
        data,
    });
}

/// Build edit deltas between two semantic token payloads.
pub(super) fn semantic_tokens_edits(
    previous: &[lsp::SemanticToken],
    next: &[lsp::SemanticToken],
) -> Vec<lsp::SemanticTokensEdit> {
    if previous == next {
        return Vec::new();
    }

    let ops = semantic_tokens_diff_ops(previous, next);
    let mut edits = Vec::new();

    let mut cursor = 0usize;
    let mut pending_start = None;
    let mut pending_delete = 0usize;
    let mut pending_data = Vec::new();
    let mut pending_cursor = 0usize;

    for op in ops {
        match op {
            SemanticTokensDiffOp::Equal => {
                if let Some(start) = pending_start {
                    push_semantic_tokens_edit(&mut edits, start, pending_delete, &mut pending_data);
                    cursor = pending_cursor;
                    pending_start = None;
                    pending_delete = 0;
                }
                cursor += 1;
            }
            SemanticTokensDiffOp::Delete => {
                if pending_start.is_none() {
                    pending_start = Some(cursor);
                    pending_cursor = cursor;
                }
                pending_delete += 1;
            }
            SemanticTokensDiffOp::Insert(token) => {
                if pending_start.is_none() {
                    pending_start = Some(cursor);
                    pending_cursor = cursor;
                }
                pending_data.push(token);
                pending_cursor += 1;
            }
        }
    }

    if let Some(start) = pending_start {
        push_semantic_tokens_edit(&mut edits, start, pending_delete, &mut pending_data);
    }

    edits
}

#[cfg(test)]
mod tests {
    use destack_lsp_types as lsp;

    use super::semantic_tokens_edits;

    /// Build a deterministic semantic token for tests.
    fn token(line: u32, start: u32, length: u32, token_type: u32) -> lsp::SemanticToken {
        lsp::SemanticToken {
            delta_line: line,
            delta_start: start,
            length,
            token_type,
            token_modifiers_bitset: 0,
        }
    }

    /// Apply semantic token edits to a payload.
    fn apply_edits(
        mut tokens: Vec<lsp::SemanticToken>,
        edits: &[lsp::SemanticTokensEdit],
    ) -> Vec<lsp::SemanticToken> {
        for edit in edits {
            let start = edit.start as usize;
            let delete_count = edit.delete_count as usize;
            let data = edit.data.clone().unwrap_or_default();
            tokens.splice(start..start + delete_count, data);
        }
        tokens
    }

    /// Apply edits with insertion only.
    #[test]
    fn test_semantic_tokens_edits_insertion() {
        let previous = vec![token(0, 0, 1, 0), token(0, 2, 1, 0)];
        let next = vec![token(0, 0, 1, 0), token(0, 1, 1, 1), token(0, 2, 1, 0)];
        let edits = semantic_tokens_edits(&previous, &next);
        let applied = apply_edits(previous, &edits);
        assert_eq!(applied, next);
    }

    /// Apply edits with deletion only.
    #[test]
    fn test_semantic_tokens_edits_deletion() {
        let previous = vec![token(0, 0, 1, 0), token(0, 1, 1, 1), token(0, 2, 1, 0)];
        let next = vec![token(0, 0, 1, 0), token(0, 2, 1, 0)];
        let edits = semantic_tokens_edits(&previous, &next);
        let applied = apply_edits(previous, &edits);
        assert_eq!(applied, next);
    }

    /// Apply edits with mixed changes.
    #[test]
    fn test_semantic_tokens_edits_multiple_changes() {
        let previous = vec![token(0, 0, 1, 0), token(0, 1, 1, 1)];
        let next = vec![token(0, 0, 1, 0), token(0, 1, 1, 2), token(0, 2, 1, 3)];
        let edits = semantic_tokens_edits(&previous, &next);
        let applied = apply_edits(previous, &edits);
        assert_eq!(applied, next);
    }
}
