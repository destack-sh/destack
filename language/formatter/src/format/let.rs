#[cfg(test)]
mod tests {
    use crate::TestFormatter;
    use crate::{DystFormatOptions, assert_format};
    use dyst_ast::DeclarationDescriptor;

    #[test]
    fn test_format_let_with_value() {
        assert_format!("let x = 1", "let x = 1", |p| p
            .eat_let(DeclarationDescriptor::default()));
    }

    #[test]
    fn test_format_let_breaks_if_too_long() {
        assert_format!(
            "const veryLongIdentifierName = veryLongIdentifierNameWithManyWords\n",
            "const veryLongIdentifierName =\n\tveryLongIdentifierNameWithManyWords\n",
            |p| p.eat_let(DeclarationDescriptor::default()),
            DystFormatOptions::default_tab().with_line_width(40)
        );
    }

    #[test]
    fn test_format_let_retain_multiline_tuple_literal() {
        let source = r"const shapes = (
    TetrisPieceShape.I,
    TetrisPieceShape.J,
    TetrisPieceShape.L,
    TetrisPieceShape.O,
)";
        assert_format!(
            source,
            source,
            |p| p.eat_let(DeclarationDescriptor::default()),
            DystFormatOptions::default_with_line_width(40)
        );
    }

    #[test]
    fn test_format_let_break_multiline_tuple_literal() {
        assert_format!(
            "const shapes = (TetrisPieceShape.I, TetrisPieceShape.J, TetrisPieceShape.L, TetrisPieceShape.O)",
            r"const shapes = (
    TetrisPieceShape.I,
    TetrisPieceShape.J,
    TetrisPieceShape.L,
    TetrisPieceShape.O,
)",
            |p| p.eat_let(DeclarationDescriptor::default()),
            DystFormatOptions::default_with_line_width(40)
        );
    }

    #[test]
    fn test_format_let_retain_multiline_array_literal() {
        let source = r"const shapes = [
    TetrisPieceShape.I,
    TetrisPieceShape.J,
    TetrisPieceShape.L,
    TetrisPieceShape.O,
]";
        assert_format!(
            source,
            source,
            |p| p.eat_let(DeclarationDescriptor::default()),
            DystFormatOptions::default_with_line_width(40)
        );
    }

    #[test]
    fn test_format_let_break_multiline_array_literal() {
        assert_format!(
            r"const shapes = [TetrisPieceShape.I, TetrisPieceShape.J, TetrisPieceShape.L, TetrisPieceShape.O]",
            r"const shapes = [
    TetrisPieceShape.I,
    TetrisPieceShape.J,
    TetrisPieceShape.L,
    TetrisPieceShape.O,
]",
            |p| p.eat_let(DeclarationDescriptor::default()),
            DystFormatOptions::default_with_line_width(40)
        );
    }

    #[test]
    fn test_format_let_multiline_if_else() {
        let source = r"const shapes = if self.nextPiece {
    const nextShape = next.shape;
    self.nextPiece = TetrisPiece.new();
    nextShape
} else {
    self.nextPiece = TetrisPiece.new();
    TetrisGame.getRandomShape(random)
}";
        assert_format!(
            source,
            source,
            |p| p.eat_let(DeclarationDescriptor::default()),
            DystFormatOptions::default_with_line_width(40)
        );
    }
}
