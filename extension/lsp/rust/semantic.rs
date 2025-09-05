//! SemanticTokens.

use crate::protocol::types as lsp;
use destack_language_token::{TokenType, tokenize};

/// Get semantic tokens for the given text.
pub fn get_semantic_tokens(text: &str) -> Vec<lsp::SemanticToken> {
    // precompute line start offsets for position calculations
    let mut line_starts: Vec<usize> = vec![0];
    for (i, ch) in text.char_indices() {
        if ch == '\n' {
            line_starts.push(i + 1);
        }
    }

    let iter = tokenize(text).peekable();
    let mut out: Vec<lsp::SemanticToken> = Vec::new();
    let mut byte_offset: usize = 0;
    let mut prev_line: u32 = 0;
    let mut prev_col: u32 = 0;

    for token in iter {
        let kind = token.r#type;
        let length = token.len as usize;

        // current token spans [byte_offset, byte_offset + length)
        let start = byte_offset;
        let end = start + length;

        // compute line (0-based) and UTF-16 column for start position
        let (line, line_start_byte) = byte_to_line_and_start(start, &line_starts);
        let col_utf16 = text[line_start_byte..start].encode_utf16().count();

        // map token to semantic type index and character length
        if let Some((token_type_index, char_len)) = map_token(&text[start..end], kind) {
            // calculate deltas relative to previous token position
            let delta_line_u32 = (line as u32).saturating_sub(prev_line);
            let delta_start_u32 = if delta_line_u32 == 0 {
                (col_utf16 as u32).saturating_sub(prev_col)
            } else {
                col_utf16 as u32
            };

            // make semantic token
            let length_u32 = char_len as u32;
            let token_modifiers_bitset = 0u32;
            out.push(lsp::SemanticToken {
                delta_line: delta_line_u32,
                delta_start: delta_start_u32,
                length: length_u32,
                token_type: token_type_index,
                token_modifiers_bitset,
            });

            prev_line = line as u32;
            prev_col = col_utf16 as u32;
        }

        byte_offset += length;
    }

    out
}

/// Get the underlying lexer token type at a given LSP position if one exists.
pub fn get_token_type_at_position(text: &str, position: &lsp::Position) -> Option<TokenType> {
    // compute line starts for mapping
    let mut line_starts: Vec<usize> = vec![0];
    for (i, ch) in text.char_indices() {
        if ch == '\n' {
            line_starts.push(i + 1);
        }
    }

    // map LSP position (UTF-16 column) to byte offset in text
    let line_index = position.line as usize;
    if line_index >= line_starts.len() {
        return None;
    }
    let line_start = line_starts[line_index];
    let line_end = if line_index + 1 < line_starts.len() {
        line_starts[line_index + 1] - 1 // exclude the newline
    } else {
        text.len()
    };

    let target_utf16_col = position.character as usize;
    let mut byte_cursor = line_start;
    let mut utf16_col_so_far: usize = 0;
    for (idx, ch) in text[line_start..line_end].char_indices() {
        if utf16_col_so_far >= target_utf16_col {
            break;
        }
        utf16_col_so_far += ch.encode_utf16(&mut [0u16; 2]).len();
        byte_cursor = line_start + idx + ch.len_utf8();
    }
    // if target col points exactly at start of line, keep line_start
    let byte_offset = if target_utf16_col == 0 {
        line_start
    } else {
        byte_cursor
    };

    // scan tokens until we cover the byte offset
    let iter = tokenize(text);
    let mut running_offset: usize = 0;
    for tok in iter {
        let start = running_offset;
        let end = start + (tok.len as usize);
        if byte_offset >= start && byte_offset < end {
            return Some(tok.r#type);
        }
        running_offset = end;
    }

    None
}

/// Find the line number and line start byte offset for a given byte index.
fn byte_to_line_and_start(byte_index: usize, line_starts: &[usize]) -> (usize, usize) {
    // binary search for the last line start <= byte_index
    let mut low = 0;
    let mut high = line_starts.len();

    while low + 1 < high {
        let mid = (low + high) / 2;
        if line_starts[mid] <= byte_index {
            low = mid;
        } else {
            high = mid;
        }
    }

    let line = low;
    let line_start = line_starts[line];
    (line, line_start)
}

/// Map a token to its semantic type index and character length.
/// TODO: replace Semantic token keywords with proper AST parsing
fn map_token(slice: &str, kind: TokenType) -> Option<(u32, usize)> {
    use TokenType as K;

    // legend indices must match `initialize` legend order
    let ty_index = match kind {
        // skip these token types first
        K::Newline | K::Whitespace | K::Unknown | K::End => return None,

        // comments
        K::LineComment | K::BlockComment => 0,
        K::DocLineComment | K::DocBlockComment => 3,

        // identifiers
        K::Identifier | K::InvalidIdentifier => 6,

        K::UnknownLiteralPrefix => 6,

        // literals
        K::Literal => 3,

        // punctuation
        K::Colon
        | K::DoubleColon
        | K::Semicolon
        | K::Comma
        | K::Dot
        | K::Range
        | K::Ellipsis
        | K::OpenParenthesis
        | K::CloseParenthesis
        | K::OpenBrace
        | K::CloseBrace
        | K::OpenBracket
        | K::CloseBracket
        | K::At
        | K::Pound
        | K::BitwiseNot
        | K::Question
        | K::Dollar => 5,

        // operators
        K::Bang
        | K::Empty
        | K::LessThan
        | K::LessThanOrEqual
        | K::GreaterThan
        | K::GreaterThanOrEqual
        | K::Equal
        | K::NotEqual
        | K::LogicalAnd
        | K::LogicalOr
        | K::LogicalAndAssign
        | K::LogicalOrAssign
        | K::BitwiseOr
        | K::BitwiseOrAssign
        | K::BitwiseAnd
        | K::BitwiseAndAssign
        | K::BitwiseXor
        | K::BitwiseXorAssign
        | K::ShiftLeft
        | K::SaturatingShiftLeft
        | K::ShiftLeftAssign
        | K::SaturatingShiftLeftAssign
        | K::ShiftRight
        | K::ShiftRightAssign
        | K::Add
        | K::WrappingAdd
        | K::SaturatingAdd
        | K::AddAssign
        | K::WrappingAddAssign
        | K::SaturatingAddAssign
        | K::Subtract
        | K::WrappingSubtract
        | K::SaturatingSubtract
        | K::SubtractAssign
        | K::WrappingSubtractAssign
        | K::SaturatingSubtractAssign
        | K::Multiply
        | K::WrappingMultiply
        | K::SaturatingMultiply
        | K::MultiplyAssign
        | K::WrappingMultiplyAssign
        | K::SaturatingMultiplyAssign
        | K::Divide
        | K::DivideAssign
        | K::Remainder
        | K::RemainderAssign => 4,

        // handle remaining tokens we don't classify yet
        _ => return None,
    };

    // length in UTF-16 code units
    Some((ty_index as u32, slice.encode_utf16().count()))
}
