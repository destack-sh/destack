use crate::token::{Token, TokenType};

/// Renders a token stream back to a string.
/// The objective is perfect roundtripping.
pub fn render_tokens(tokens: &[Token], source: &str) -> String {
    let mut out = String::new();
    let mut offset: usize = 0;
    for tok in tokens {
        let len = tok.len as usize;
        match tok.r#type {
            TokenType::LineComment { .. }
            | TokenType::Whitespace
            | TokenType::Identifier
            | TokenType::InvalidIdentifier
            | TokenType::RawIdentifier
            | TokenType::UnknownLiteralPrefix
            | TokenType::Literal { .. } => {
                // emit original slice for lexemes where we don't want to reformat
                out.push_str(&source[offset..offset + len]);
            }
            TokenType::Semicolon => out.push(';'),
            TokenType::Comma => out.push(','),
            TokenType::Dot => out.push('.'),
            TokenType::DotDot => out.push_str(".."),
            TokenType::DotDotDot => out.push_str("..."),
            TokenType::OpenParenthesis => out.push('('),
            TokenType::CloseParenthesis => out.push(')'),
            TokenType::OpenBrace => out.push('{'),
            TokenType::CloseBrace => out.push('}'),
            TokenType::OpenBracket => out.push('['),
            TokenType::CloseBracket => out.push(']'),
            TokenType::At => out.push('@'),
            TokenType::Pound => out.push('#'),
            TokenType::Tilde => out.push('~'),
            TokenType::Question => out.push('?'),
            TokenType::Colon => out.push(':'),
            TokenType::DoubleColon => out.push_str("::"),
            TokenType::Dollar => out.push('$'),
            TokenType::Equals => out.push('='),
            TokenType::FatArrow => out.push_str("=>"),
            TokenType::Bang => out.push('!'),
            TokenType::LessThan => out.push('<'),
            TokenType::GreaterThan => out.push('>'),
            TokenType::Minus => out.push('-'),
            TokenType::ThinArrow => out.push_str("->"),
            TokenType::And => out.push('&'),
            TokenType::Or => out.push('|'),
            TokenType::Plus => out.push('+'),
            TokenType::Star => out.push('*'),
            TokenType::Slash => out.push('/'),
            TokenType::Caret => out.push('^'),
            TokenType::Percent => out.push('%'),
            TokenType::Unknown => {
                // emit original slice for unknown tokens to preserve them
                out.push_str(&source[offset..offset + len]);
            }
            TokenType::Eof => {}
        }
        offset += len;
    }
    out
}
