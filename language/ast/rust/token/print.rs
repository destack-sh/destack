use super::token::{Token, TokenType};

impl Token {
    /// Renders this token back to its string representation.
    #[inline]
    pub fn render(&self, source: &str, offset: usize) -> String {
        let len = self.len as usize;
        match self.ty {
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
            TokenType::Arrow => "->".to_string(),
            TokenType::ArrowWide => "=>".to_string(),

            // parentheses
            TokenType::OpenParenthesis => "(".to_string(),
            TokenType::CloseParenthesis => ")".to_string(),
            TokenType::OpenBrace => "{".to_string(),
            TokenType::CloseBrace => "}".to_string(),
            TokenType::OpenBracket => "[".to_string(),
            TokenType::CloseBracket => "]".to_string(),

            TokenType::At => "@".to_string(),
            TokenType::Tag => "#".to_string(),
            TokenType::ElementwiseNot => "~".to_string(),
            TokenType::Maybe => "?".to_string(),
            TokenType::Coalesce => "??".to_string(),
            TokenType::Dynamic => "$".to_string(),
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
            TokenType::Increment => "++".to_string(),
            TokenType::Decrement => "--".to_string(),

            // shift
            TokenType::ShiftLeft => "<<".to_string(),
            TokenType::SaturatingShiftLeft => "<<|".to_string(),

            // elementwise
            TokenType::ElementwiseAnd => "&".to_string(),
            TokenType::ElementwiseXor => "^".to_string(),
            TokenType::ElementwiseOr => "|".to_string(),

            // comparison
            TokenType::Equal => "==".to_string(),
            TokenType::EqualWide => "===".to_string(),
            TokenType::NotEqual => "!=".to_string(),
            TokenType::NotEqualWide => "!==".to_string(),
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

            // assignment elementwise
            TokenType::ElementwiseAndAssign => "&=".to_string(),
            TokenType::ElementwiseXorAssign => "^=".to_string(),
            TokenType::ElementwiseOrAssign => "|=".to_string(),

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
