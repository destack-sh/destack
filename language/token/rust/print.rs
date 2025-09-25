use super::token::{Token, TokenType};

impl Token {
    /// Renders this token back to its string representation.
    #[inline]
    pub fn render(&self, source: &str, offset: usize) -> String {
        let len = self.len as usize;
        match self.r#type {
            // structural
            TokenType::Newline => "\n".to_string(),
            TokenType::Whitespace => {
                // emit original slice for lexemes where we don't want to reformat
                source[offset..offset + len].to_string()
            }
            TokenType::Unknown => {
                // emit original slice for unknown tokens to preserve them
                source[offset..offset + len].to_string()
            }
            TokenType::End => String::new(),

            // annotations
            TokenType::LineComment => {
                // emit original slice for lexemes where we don't want to reformat
                source[offset..offset + len].to_string()
            }
            TokenType::BlockComment => {
                // preserve original formatting
                source[offset..offset + len].to_string()
            }
            TokenType::DocLineComment => {
                // emit original slice for lexemes where we don't want to reformat
                source[offset..offset + len].to_string()
            }
            TokenType::DocBlockComment => {
                // preserve original formatting
                source[offset..offset + len].to_string()
            }

            // identifiers / literals
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
            TokenType::Literal => {
                // emit original slice for lexemes where we don't want to reformat
                source[offset..offset + len].to_string()
            }

            // symbols
            TokenType::Wildcard => "_".to_string(),
            TokenType::Colon => ":".to_string(),
            TokenType::Semicolon => ";".to_string(),
            TokenType::Comma => ",".to_string(),
            TokenType::Dot => ".".to_string(),
            TokenType::Range => "..".to_string(),
            TokenType::RangeWide => "...".to_string(),
            TokenType::Empty => "--".to_string(),
            TokenType::EmptyWide => "---".to_string(),
            TokenType::FatArrow => "=>".to_string(),
            TokenType::ThinArrow => "->".to_string(),

            // parentheses
            TokenType::OpenParenthesis => "(".to_string(),
            TokenType::CloseParenthesis => ")".to_string(),
            TokenType::OpenBrace => "{".to_string(),
            TokenType::CloseBrace => "}".to_string(),
            TokenType::OpenBracket => "[".to_string(),
            TokenType::CloseBracket => "]".to_string(),

            TokenType::At => "@".to_string(),
            TokenType::Tag => "#".to_string(),
            TokenType::BitwiseNot => "~".to_string(),
            TokenType::Maybe => "?".to_string(),
            TokenType::Coalesce => "??".to_string(),
            TokenType::Virtual => "$".to_string(),
            TokenType::Not => "!".to_string(),

            // multiplication
            TokenType::Multiply => "*".to_string(),
            TokenType::WrappingMultiply => "*%".to_string(),
            TokenType::SaturatingMultiply => "*|".to_string(),
            TokenType::Divide => "/".to_string(),
            TokenType::Remainder => "%".to_string(),

            // addition
            TokenType::Add => "+".to_string(),
            TokenType::WrappingAdd => "+%".to_string(),
            TokenType::SaturatingAdd => "+|".to_string(),
            TokenType::Subtract => "-".to_string(),
            TokenType::WrappingSubtract => "-%".to_string(),
            TokenType::SaturatingSubtract => "-|".to_string(),

            // shift
            TokenType::ShiftLeft => "<<".to_string(),
            TokenType::SaturatingShiftLeft => "<<|".to_string(),
            TokenType::ShiftRight => ">>".to_string(),

            // bitwise
            TokenType::BitwiseAnd => "&".to_string(),
            TokenType::BitwiseXor => "^".to_string(),
            TokenType::BitwiseOr => "|".to_string(),

            // comparison
            TokenType::Equal => "==".to_string(),
            TokenType::NotEqual => "!=".to_string(),
            TokenType::LessThan => "<".to_string(),
            TokenType::LessThanOrEqual => "<=".to_string(),
            TokenType::GreaterThan => ">".to_string(),
            TokenType::GreaterThanOrEqual => ">=".to_string(),

            // logical
            TokenType::LogicalAnd => "&&".to_string(),
            TokenType::LogicalOr => "||".to_string(),

            // assignment
            TokenType::Assign => "=".to_string(),

            // assignment multiplication
            TokenType::MultiplyAssign => "*=".to_string(),
            TokenType::WrappingMultiplyAssign => "*%=".to_string(),
            TokenType::SaturatingMultiplyAssign => "*|=".to_string(),
            TokenType::DivideAssign => "/=".to_string(),
            TokenType::RemainderAssign => "%=".to_string(),

            // assignment addition
            TokenType::AddAssign => "+=".to_string(),
            TokenType::WrappingAddAssign => "+%=".to_string(),
            TokenType::SaturatingAddAssign => "+|=".to_string(),
            TokenType::SubtractAssign => "-=".to_string(),
            TokenType::WrappingSubtractAssign => "-%=".to_string(),
            TokenType::SaturatingSubtractAssign => "-|=".to_string(),

            // assignment shift
            TokenType::ShiftLeftAssign => "<<=".to_string(),
            TokenType::SaturatingShiftLeftAssign => "<<|=".to_string(),
            TokenType::ShiftRightAssign => ">>=".to_string(),

            // assignment bitwise
            TokenType::BitwiseAndAssign => "&=".to_string(),
            TokenType::BitwiseXorAssign => "^=".to_string(),
            TokenType::BitwiseOrAssign => "|=".to_string(),

            // assignment logical
            TokenType::LogicalAndAssign => "&&=".to_string(),
            TokenType::LogicalOrAssign => "||=".to_string(),
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
