use super::token::{Token, TokenType};

/// Renders a token stream back to a string.
/// The objective is perfect roundtripping.
pub fn render_tokens(tokens: &[Token], source: &str) -> String {
    let mut out = String::new();
    let mut offset: usize = 0;
    for tok in tokens {
        let len = tok.len as usize;
        match tok.r#type {
            TokenType::Whitespace => {
                // emit original slice for lexemes where we don't want to reformat
                out.push_str(&source[offset..offset + len]);
            }
            TokenType::LineComment => {
                // emit original slice for lexemes where we don't want to reformat
                out.push_str(&source[offset..offset + len]);
            }
            TokenType::Unknown => {
                // emit original slice for unknown tokens to preserve them
                out.push_str(&source[offset..offset + len]);
            }
            TokenType::EndOfInput => {}

            TokenType::DocComment => {
                // emit original slice for lexemes where we don't want to reformat
                out.push_str(&source[offset..offset + len]);
            }
            TokenType::Identifier => {
                // emit original slice for lexemes where we don't want to reformat
                out.push_str(&source[offset..offset + len]);
            }
            TokenType::InvalidIdentifier => {
                // emit original slice for lexemes where we don't want to reformat
                out.push_str(&source[offset..offset + len]);
            }
            TokenType::UnknownLiteralPrefix => {
                // emit original slice for lexemes where we don't want to reformat
                out.push_str(&source[offset..offset + len]);
            }
            TokenType::Literal { .. } => {
                // emit original slice for lexemes where we don't want to reformat
                out.push_str(&source[offset..offset + len]);
            }

            TokenType::Colon => out.push(':'),
            TokenType::Semicolon => out.push(';'),
            TokenType::Comma => out.push(','),
            TokenType::Dot => out.push('.'),
            TokenType::Range => out.push_str(".."),
            TokenType::Ellipsis => out.push_str("..."),

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
            TokenType::Dollar => out.push('$'),
            TokenType::Bang => out.push('!'),
            TokenType::LessThan => out.push('<'),
            TokenType::ShiftLeft => out.push_str("<<"),
            TokenType::ShiftLeftAssign => out.push_str("<<="),
            TokenType::GreaterThan => out.push('>'),
            TokenType::ShiftRight => out.push_str(">>"),
            TokenType::ShiftRightAssign => out.push_str(">>="),
            TokenType::Assign => out.push('='),
            TokenType::Arrow => out.push_str("=>"),
            TokenType::BadArrow => out.push_str("->"),
            TokenType::Add => out.push('+'),
            TokenType::AddAssign => out.push_str("+="),
            TokenType::Subtract => out.push('-'),
            TokenType::SubtractAssign => out.push_str("-="),
            TokenType::Multiply => out.push('*'),
            TokenType::MultiplyAssign => out.push_str("*="),
            TokenType::Divide => out.push('/'),
            TokenType::DivideAssign => out.push_str("/="),
            TokenType::Percent => out.push('%'),
            TokenType::RemainderAssign => out.push_str("%="),
            TokenType::Caret => out.push('^'),
            TokenType::ExponentAssign => out.push_str("^="),
            TokenType::BitwiseAnd => out.push('&'),
            TokenType::LogicalAnd => out.push_str("&&"),
            TokenType::BitwiseAndAssign => out.push_str("&="),
            TokenType::BitwiseOr => out.push('|'),
            TokenType::LogicalOr => out.push_str("||"),
            TokenType::BitwiseOrAssign => out.push_str("|="),
            TokenType::Empty => out.push_str("---"),
        }
        offset += len;
    }
    out
}
