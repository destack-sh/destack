//! Semantic token computation.

use tower_lsp::lsp_types as lsp;

/// Compute semantic tokens for the given text.
pub fn compute_semantic_tokens(text: &str) -> Vec<lsp::SemanticToken> {
    // precompute line start offsets for position calculations
    let mut line_starts: Vec<usize> = vec![0];
    for (i, ch) in text.char_indices() {
        if ch == '\n' {
            line_starts.push(i + 1);
        }
    }

    let iter = destack_lexer::tokenize(text).peekable();
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
fn map_token(slice: &str, kind: destack_lexer::token::TokenType) -> Option<(u32, usize)> {
    use destack_lexer::token::TokenType as K;

    // legend indices must match `initialize` legend order
    let ty_index = match kind {
        K::LineComment { .. } => 0, // COMMENT

        // keywords and identifiers
        K::Identifier | K::RawIdentifier | K::InvalidIdentifier => {
            const KEYWORDS: &[&str] = &[
                "use", "struct", "enum", "entity", "impl", "fn", "let", "return", "extends",
                "true", "false", "None", "Some",
            ];

            if KEYWORDS.contains(&slice) {
                1 // KEYWORD
            } else {
                // crude heuristic: UpperCamelCase => TYPE, otherwise VARIABLE
                let is_type_like = slice
                    .chars()
                    .next()
                    .map(|c| c.is_ascii_uppercase())
                    .unwrap_or(false);
                if is_type_like { 6 } else { 7 }
            }
        }

        K::UnknownLiteralPrefix => 5, // map to FUNCTION color (distinct)

        // literals
        K::Literal { r#type, .. } => match r#type {
            destack_lexer::token::LiteralTokenType::Integer { .. }
            | destack_lexer::token::LiteralTokenType::Float { .. } => 3, // NUMBER

            destack_lexer::token::LiteralTokenType::Character { .. }
            | destack_lexer::token::LiteralTokenType::Byte { .. }
            | destack_lexer::token::LiteralTokenType::String { .. }
            | destack_lexer::token::LiteralTokenType::ByteString { .. }
            | destack_lexer::token::LiteralTokenType::RawString { .. }
            | destack_lexer::token::LiteralTokenType::RawByteString { .. } => 2, // STRING
        },

        // punctuation - use FUNCTION color to differentiate from operators
        K::Semicolon
        | K::Comma
        | K::Dot
        | K::DotDot
        | K::DotDotDot
        | K::OpenParenthesis
        | K::CloseParenthesis
        | K::OpenBrace
        | K::CloseBrace
        | K::OpenBracket
        | K::CloseBracket
        | K::At
        | K::Pound
        | K::Tilde
        | K::Question
        | K::Colon
        | K::DoubleColon
        | K::Dollar => 5, // FUNCTION color to differentiate

        // operators
        K::Equals
        | K::FatArrow
        | K::Bang
        | K::LessThan
        | K::GreaterThan
        | K::Minus
        | K::ThinArrow
        | K::And
        | K::Or
        | K::Plus
        | K::Star
        | K::Slash
        | K::Caret
        | K::Percent => 4, // OPERATOR

        // skip these token types
        K::Whitespace | K::Unknown | K::Eof => return None,
    };

    // length in UTF-16 code units
    Some((ty_index as u32, slice.encode_utf16().count()))
}
