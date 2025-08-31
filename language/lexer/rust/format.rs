use super::token::{Token, TokenType};

impl Token {
    /// Renders this token back to its string representation.
    #[inline]
    pub fn render(&self, source: &str, offset: usize) -> String {
        let len = self.len as usize;
        match self.r#type {
            TokenType::Newline => "\n".to_string(),
            TokenType::Whitespace => {
                // emit original slice for lexemes where we don't want to reformat
                source[offset..offset + len].to_string()
            }
            TokenType::LineComment => {
                // emit original slice for lexemes where we don't want to reformat
                source[offset..offset + len].to_string()
            }
            TokenType::Unknown => {
                // emit original slice for unknown tokens to preserve them
                source[offset..offset + len].to_string()
            }
            TokenType::End => String::new(),

            TokenType::DocComment => {
                // emit original slice for lexemes where we don't want to reformat
                source[offset..offset + len].to_string()
            }
            TokenType::Identifier => {
                // emit original slice for lexemes where we don't want to reformat
                source[offset..offset + len].to_string()
            }
            TokenType::InvalidIdentifier => {
                // emit original slice for lexemes where we don't want to reformat
                source[offset..offset + len].to_string()
            }
            TokenType::UnknownLiteralPrefix => {
                // emit original slice for lexemes where we don't want to reformat
                source[offset..offset + len].to_string()
            }
            TokenType::Literal(_) => {
                // emit original slice for lexemes where we don't want to reformat
                source[offset..offset + len].to_string()
            }

            TokenType::Colon => ":".to_string(),
            TokenType::Semicolon => ";".to_string(),
            TokenType::Comma => ",".to_string(),
            TokenType::Dot => ".".to_string(),
            TokenType::Range => "..".to_string(),
            TokenType::Ellipsis => "...".to_string(),

            TokenType::OpenParenthesis => "(".to_string(),
            TokenType::CloseParenthesis => ")".to_string(),
            TokenType::OpenBrace => "{".to_string(),
            TokenType::CloseBrace => "}".to_string(),
            TokenType::OpenBracket => "[".to_string(),
            TokenType::CloseBracket => "]".to_string(),

            TokenType::At => "@".to_string(),
            TokenType::Pound => "#".to_string(),
            TokenType::Tilde => "~".to_string(),
            TokenType::Question => "?".to_string(),
            TokenType::Dollar => "$".to_string(),
            TokenType::Bang => "!".to_string(),
            TokenType::Empty => "---".to_string(),
            TokenType::LogicalAnd => "&&".to_string(),
            TokenType::LogicalOr => "||".to_string(),
            TokenType::Arrow => "=>".to_string(),
            TokenType::BadArrow => "->".to_string(),

            // Assignment
            TokenType::Assign => "=".to_string(),

            // Comparison
            TokenType::GreaterThan => ">".to_string(),
            TokenType::LessThan => "<".to_string(),
            TokenType::GreaterThanEqual => ">=".to_string(),
            TokenType::LessThanEqual => "<=".to_string(),
            TokenType::Equal => "==".to_string(),
            TokenType::NotEqual => "!=".to_string(),

            // Bitwise (assignment form)
            TokenType::BitwiseOr => "|".to_string(),
            TokenType::BitwiseOrAssign => "|=".to_string(),
            TokenType::BitwiseAnd => "&".to_string(),
            TokenType::BitwiseAndAssign => "&=".to_string(),
            TokenType::BitwiseXor => "^".to_string(),
            TokenType::BitwiseXorAssign => "^=".to_string(),
            TokenType::ShiftLeft => "<<".to_string(),
            TokenType::ShiftLeftAssign => "<<=".to_string(),
            TokenType::SaturatingShiftLeft => "<<|".to_string(),
            TokenType::SaturatingShiftLeftAssign => "<<|=".to_string(),
            TokenType::ShiftRight => ">>".to_string(),
            TokenType::ShiftRightAssign => ">>=".to_string(),

            // Arithmetic (assignment form)
            TokenType::Add => "+".to_string(),
            TokenType::AddAssign => "+=".to_string(),
            TokenType::WrappingAdd => "+%".to_string(),
            TokenType::WrappingAddAssign => "+%=".to_string(),
            TokenType::SaturatingAdd => "+|".to_string(),
            TokenType::SaturatingAddAssign => "+|=".to_string(),
            TokenType::Subtract => "-".to_string(),
            TokenType::SubtractAssign => "-=".to_string(),
            TokenType::WrappingSubtract => "-%".to_string(),
            TokenType::WrappingSubtractAssign => "-%=".to_string(),
            TokenType::SaturatingSubtract => "-|".to_string(),
            TokenType::SaturatingSubtractAssign => "-|=".to_string(),
            TokenType::Multiply => "*".to_string(),
            TokenType::MultiplyAssign => "*=".to_string(),
            TokenType::WrappingMultiply => "*%".to_string(),
            TokenType::WrappingMultiplyAssign => "*%=".to_string(),
            TokenType::SaturatingMultiply => "*|".to_string(),
            TokenType::SaturatingMultiplyAssign => "*|=".to_string(),
            TokenType::Divide => "/".to_string(),
            TokenType::DivideAssign => "/=".to_string(),
            TokenType::Remainder => "%".to_string(),
            TokenType::RemainderAssign => "%=".to_string(),
        }
    }
}

/// Renders a token stream back to a string.
/// The objective is perfect roundtripping.
pub fn render_tokens(tokens: &[Token], source: &str) -> String {
    let mut out = String::new();
    let mut offset: usize = 0;
    for tok in tokens {
        let len = tok.len as usize;
        out.push_str(&tok.render(source, offset));
        offset += len;
    }
    out
}
