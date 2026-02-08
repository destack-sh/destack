#[cfg(test)]
mod tests {
    use crate::{DestackFormatOptions, TestFormatter, assert_format};
    use destack_ast::{Asynchrony, DeclarationDescriptor};
    use destack_source::FileType;

    #[test]
    fn test_format_let_with_value() {
        assert_format!("let x = 1", "let x = 1", |p| p
            .eat_let(&p.mark(), DeclarationDescriptor::default()));
    }

    #[test]
    fn test_format_using_with_value() {
        assert_format!("using x = open()", "using x = open()", |p| p.eat_using(
            &p.mark(),
            DeclarationDescriptor::default(),
            Asynchrony::Sync
        ));
    }

    #[test]
    fn test_format_await_using_with_value() {
        assert_format!(
            "await using conn = open()",
            "await using conn = open()",
            |p| p.eat_using(
                &p.mark(),
                DeclarationDescriptor::default(),
                Asynchrony::Async
            )
        );
    }

    #[test]
    fn test_format_let_breaks_if_too_long() {
        assert_format!(
            "const veryLongIdentifierName = veryLongIdentifierNameWithManyWords\n",
            "const veryLongIdentifierName =\n\tveryLongIdentifierNameWithManyWords\n",
            |p| p.eat_let(&p.mark(), DeclarationDescriptor::default()),
            DestackFormatOptions::default_tab().with_line_width(40)
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
            |p| p.eat_let(&p.mark(), DeclarationDescriptor::default()),
            DestackFormatOptions::default_with_line_width(40)
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
            |p| p.eat_let(&p.mark(), DeclarationDescriptor::default()),
            DestackFormatOptions::default_with_line_width(40)
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
            |p| p.eat_let(&p.mark(), DeclarationDescriptor::default()),
            DestackFormatOptions::default_with_line_width(40)
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
            |p| p.eat_let(&p.mark(), DeclarationDescriptor::default()),
            DestackFormatOptions::default_with_line_width(40)
        );
    }

    #[test]
    fn test_format_let_multiline_if_else() {
        let source = r"const shapes = if (self.nextPiece) {
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
            |p| p.eat_let(&p.mark(), DeclarationDescriptor::default()),
            DestackFormatOptions::default_with_line_width(40)
        );
    }

    #[test]
    fn test_format_let_destructuring_pattern_expanded() {
        assert_format!(
            r#"const { name = "default", count: total = 0, items: [...rest] } = config"#,
            r#"const {
    name = "default",
    count: total = 0,
    items: [...rest],
} = config"#,
            |p| p.eat_let(&p.mark(), DeclarationDescriptor::default()),
            DestackFormatOptions::default_with_line_width(60)
        );
    }

    #[test]
    fn test_format_let_ignored_binding_field_is_idempotent() {
        let source = "let {\n\t/* biome-ignore format: Test that the property doesn't get formatted */\n\tsomeProperty:    alias\n} = { someProperty: 20 };";
        let (first_test, first_node_id) =
            TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| {
                p.eat_let(&p.mark(), DeclarationDescriptor::default())
            })
            .unwrap();
        let first_output = first_test.format(
            &first_node_id,
            DestackFormatOptions::default_with_line_width(80),
        );

        let (second_test, second_node_id) =
            TestFormatter::parse_with_file_type(&first_output, FileType::TypeScript, |p| {
                p.eat_let(&p.mark(), DeclarationDescriptor::default())
            })
            .unwrap();
        let second_output = second_test.format(
            &second_node_id,
            DestackFormatOptions::default_with_line_width(80),
        );

        assert_eq!(first_output, second_output);
        assert!(!second_output.contains(",,"));
    }

    #[test]
    fn test_format_let_multiline_chain_rhs_is_idempotent() {
        let source = "const logical_expression_1 = this.state\n  .longLongLongLongLongLongLongLongLongTooLongProp\n  === true;";
        let (first_test, first_node_id) =
            TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| {
                p.eat_let(&p.mark(), DeclarationDescriptor::default())
            })
            .unwrap();
        let first_output = first_test.format(
            &first_node_id,
            DestackFormatOptions::default_with_line_width(80),
        );

        let (second_test, second_node_id) =
            TestFormatter::parse_with_file_type(&first_output, FileType::TypeScript, |p| {
                p.eat_let(&p.mark(), DeclarationDescriptor::default())
            })
            .unwrap();
        let second_output = second_test.format(
            &second_node_id,
            DestackFormatOptions::default_with_line_width(80),
        );

        assert_eq!(first_output, second_output);
    }
}
