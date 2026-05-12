use super::token::{Token, TokenType};

impl Token {
    /// Renders this token back to its string representation.
    #[inline]
    pub fn render(&self, source: &str, offset: usize) -> String {
        let len = self.len as usize;
        match self.ty {
            // --------------------------------------------------
            // Structural
            // --------------------------------------------------
            TokenType::Newline => "\n".to_string(),
            TokenType::Whitespace => source[offset..offset + len].to_string(),
            TokenType::Unknown => source[offset..offset + len].to_string(),
            TokenType::End => String::new(),

            // --------------------------------------------------
            // Annotations
            // --------------------------------------------------
            TokenType::LineComment => source[offset..offset + len].to_string(),
            TokenType::BlockComment => source[offset..offset + len].to_string(),
            TokenType::DocLineComment => source[offset..offset + len].to_string(),
            TokenType::DocBlockComment => source[offset..offset + len].to_string(),

            // --------------------------------------------------
            // Identifiers
            // --------------------------------------------------
            TokenType::Identifier => source[offset..offset + len].to_string(),
            TokenType::InvalidIdentifier => source[offset..offset + len].to_string(),

            // --------------------------------------------------
            // Literals & Prefixes
            // --------------------------------------------------
            TokenType::UnknownLiteralPrefix => source[offset..offset + len].to_string(),
            TokenType::Literal => source[offset..offset + len].to_string(),
            TokenType::TemplateStringStart => source[offset..offset + len].to_string(),
            TokenType::TemplateStringMiddle => source[offset..offset + len].to_string(),
            TokenType::TemplateStringEnd => source[offset..offset + len].to_string(),
            TokenType::TemplateString => source[offset..offset + len].to_string(),

            // --------------------------------------------------
            // Symbols
            // --------------------------------------------------
            TokenType::Colon => ":".to_string(),
            TokenType::Semicolon => ";".to_string(),
            TokenType::Comma => ",".to_string(),
            TokenType::Dot => ".".to_string(),
            TokenType::Spread => "...".to_string(),
            TokenType::Arrow => "->".to_string(),
            TokenType::ArrowWide => "=>".to_string(),

            // --------------------------------------------------
            // Grouping
            // --------------------------------------------------
            TokenType::OpenParenthesis => "(".to_string(),
            TokenType::CloseParenthesis => ")".to_string(),
            TokenType::OpenBrace => "{".to_string(),
            TokenType::CloseBrace => "}".to_string(),
            TokenType::OpenBracket => "[".to_string(),
            TokenType::CloseBracket => "]".to_string(),

            // --------------------------------------------------
            // Misc
            // --------------------------------------------------
            TokenType::At => "@".to_string(),
            TokenType::Hash => "#".to_string(),

            // --------------------------------------------------
            // Elementwise / Logical / Dynamic prefixes
            // --------------------------------------------------
            TokenType::ElementwiseNot => "~".to_string(),
            TokenType::Maybe => "?".to_string(),
            TokenType::Coalesce => "??".to_string(),
            TokenType::Not => "!".to_string(),

            // --------------------------------------------------
            // Operators
            // --------------------------------------------------
            // Multiplication, Division, Remainder
            TokenType::Multiply => "*".to_string(),
            TokenType::Exponent => "**".to_string(),
            TokenType::Divide => "/".to_string(),
            TokenType::Remainder => "%".to_string(),

            // Addition, Subtraction
            TokenType::Add => "+".to_string(),
            TokenType::Subtract => "-".to_string(),

            // Increment/Decrement
            TokenType::Increment => "++".to_string(),
            TokenType::Decrement => "--".to_string(),

            // Shifts
            TokenType::ShiftLeft => "<<".to_string(),
            TokenType::ShiftRight => ">>".to_string(),
            TokenType::UnsignedShiftRight => ">>>".to_string(),

            // Elementwise
            TokenType::ElementwiseAnd => "&".to_string(),
            TokenType::ElementwiseXor => "^".to_string(),
            TokenType::ElementwiseOr => "|".to_string(),

            // Comparison
            TokenType::Equal => "==".to_string(),
            TokenType::EqualWide => "===".to_string(),
            TokenType::NotEqual => "!=".to_string(),
            TokenType::NotEqualWide => "!==".to_string(),
            TokenType::LessThan => "<".to_string(),
            TokenType::LessThanOrEqual => "<=".to_string(),
            TokenType::GreaterThan => ">".to_string(),
            TokenType::GreaterThanOrEqual => ">=".to_string(),

            // Logical
            TokenType::LogicalAnd => "&&".to_string(),
            TokenType::LogicalOr => "||".to_string(),

            // --------------------------------------------------
            // Assignment Operators
            // --------------------------------------------------
            // Assignment
            TokenType::Assign => "=".to_string(),

            // Multiplication assignment
            TokenType::MultiplyAssign => "*=".to_string(),
            TokenType::ExponentAssign => "**=".to_string(),
            TokenType::DivideAssign => "/=".to_string(),
            TokenType::RemainderAssign => "%=".to_string(),

            // Addition assignment
            TokenType::AddAssign => "+=".to_string(),
            TokenType::SubtractAssign => "-=".to_string(),

            // Shift assignment
            TokenType::ShiftLeftAssign => "<<=".to_string(),
            TokenType::ShiftRightAssign => ">>=".to_string(),
            TokenType::UnsignedShiftRightAssign => ">>>=".to_string(),

            // Elementwise assignment
            TokenType::ElementwiseAndAssign => "&=".to_string(),
            TokenType::ElementwiseXorAssign => "^=".to_string(),
            TokenType::ElementwiseOrAssign => "|=".to_string(),

            // Logical assignment
            TokenType::LogicalAndAssign => "&&=".to_string(),
            TokenType::LogicalOrAssign => "||=".to_string(),
            TokenType::CoalesceAssign => "??=".to_string(),
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
