mod extension {
    use crate::{DestackFormatOptions, TestFormatter, assert_format};
    use destack_ast::DeclarationDescriptor;

    #[test]
    fn test_format_extension_empty() {
        assert_format!(
            "extension for Foo {}",
            "extension for Foo {}",
            |p| p.eat_extension(&p.mark(), DeclarationDescriptor::default()),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_extension_with_implements() {
        assert_format!(
            "extension for Foo implements Bar { static X = 1 }",
            "extension for Foo implements Bar {\n\tstatic X = 1;\n}",
            |p| p.eat_extension(&p.mark(), DeclarationDescriptor::default()),
            DestackFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_extension_with_static_arguments() {
        assert_format!(
            "extension<T> for Foo<T> { }",
            "extension<T> for Foo<T> {}",
            |p| p.eat_extension(&p.mark(), DeclarationDescriptor::default()),
            DestackFormatOptions::default()
        );
    }
}

mod function {
    use crate::{DestackFormatOptions, TestFormatter, assert_format};
    use destack_ast::DeclarationDescriptor;

    #[test]
    fn test_format_function_lambda_empty() {
        assert_format!("(): void => {}", "(): void => {}", |p| p.eat_function(
            &p.mark(),
            DeclarationDescriptor::default(),
            false,
            false
        ));
    }

    #[test]
    fn test_format_function_lambda_with_parameters() {
        assert_format!("(a: int32) => a > 2", "(a: int32) => a > 2", |p| p
            .eat_function(
                &p.mark(),
                DeclarationDescriptor::default(),
                false,
                false
            ));
    }

    #[test]
    fn test_format_function_simple() {
        assert_format!("function foo() {}", "function foo() {}", |p| p
            .eat_function(
                &p.mark(),
                DeclarationDescriptor::default(),
                false,
                false
            ));
    }

    #[test]
    fn test_format_function_with_parameters() {
        assert_format!(
            "function bar(x: int32, y: boolean) {}",
            "function bar(x: int32, y: boolean) {}",
            |p| p.eat_function(&p.mark(), DeclarationDescriptor::default(), false, false)
        );
    }

    #[test]
    fn test_format_function_with_parameters_overflow() {
        assert_format!(
            "function bar(x: int32, y: boolean, z: string) {}",
            "function bar(\n\tx: int32,\n\ty: boolean,\n\tz: string,\n) {}",
            |p| p.eat_function(&p.mark(), DeclarationDescriptor::default(), false, false),
            DestackFormatOptions::default_tab_with_line_width(40)
        );
    }

    #[test]
    fn test_format_function_with_return_type() {
        assert_format!(
            "function baz(): int32 {}",
            "function baz(): int32 {}",
            |p| p.eat_function(&p.mark(), DeclarationDescriptor::default(), false, false)
        );
    }

    #[test]
    fn test_format_function_with_static_parameters() {
        assert_format!(
            "function generic<T, U>() {}",
            "function generic<T, U>() {}",
            |p| p.eat_function(&p.mark(), DeclarationDescriptor::default(), false, false)
        );
    }

    #[test]
    fn test_format_function_declaration() {
        assert_format!(
            "function external(): int32",
            "function external(): int32;",
            |p| p.eat_function(&p.mark(), DeclarationDescriptor::default(), false, false)
        );
    }

    #[test]
    fn test_format_function_with_self_parameter() {
        let source = r"function foo(self: int32): void";
        assert_format!(source, "function foo(self: int32): void;", |p| p
            .eat_function(
                &p.mark(),
                DeclarationDescriptor::default(),
                false,
                false
            ));
    }

    #[test]
    fn test_format_function_with_this_parameter() {
        let source = r"function foo(this: int32): void";
        assert_format!(source, "function foo(this: int32): void;", |p| p
            .eat_function(
                &p.mark(),
                DeclarationDescriptor::default(),
                false,
                false
            ));
    }

    #[test]
    fn test_format_function_with_type_predicate() {
        let source = r"function assertFoo(value: Foo): asserts value is Foo";
        assert_format!(
            source,
            "function assertFoo(value: Foo): asserts value is Foo;",
            |p| p.eat_function(&p.mark(), DeclarationDescriptor::default(), false, false)
        );
    }

    #[test]
    fn test_format_function_with_type_predicate_asserts_value() {
        let source = r"function assertFoo(value: Foo): asserts value";
        assert_format!(
            source,
            "function assertFoo(value: Foo): asserts value;",
            |p| p.eat_function(&p.mark(), DeclarationDescriptor::default(), false, false)
        );
    }

    #[test]
    fn test_format_function_with_type_predicate_this() {
        let source = r"function assertFoo(this: Foo): asserts this is Foo";
        assert_format!(
            source,
            "function assertFoo(this: Foo): asserts this is Foo;",
            |p| p.eat_function(&p.mark(), DeclarationDescriptor::default(), false, false)
        );
    }

    #[test]
    fn test_format_function_with_type_predicate_this_no_target() {
        let source = r"function assertFoo(this: Foo): asserts this";
        assert_format!(
            source,
            "function assertFoo(this: Foo): asserts this;",
            |p| p.eat_function(&p.mark(), DeclarationDescriptor::default(), false, false)
        );
    }

    #[test]
    fn test_format_function_with_self_return_type() {
        let source = r"function init(capacity: int32): Self {
    Self {
        map: Map.new(capacity),
        queue: Queue.new(capacity),
        capacity: capacity,
        somethingElse: something,
        moreStuff: bar(),
        evenMoreStuff: foo(),
        moreMoreMoreStuff: baz(),
    }
}";
        assert_format!(
            source,
            r"function init(capacity: int32): Self {
    Self {
        map: Map.new(capacity),
        queue: Queue.new(capacity),
        capacity: capacity,
        somethingElse: something,
        moreStuff: bar(),
        evenMoreStuff: foo(),
        moreMoreMoreStuff: baz(),
    }
}",
            |p| p.eat_function(&p.mark(), DeclarationDescriptor::default(), false, false)
        );
    }
}

mod if_statement {
    use crate::{DestackFormatOptions, TestFormatter, assert_format};

    #[test]
    fn test_format_if_with_body() {
        assert_format!(
            "if (cond) { const X = 1 } else { const Y = 2 }",
            "if (cond) {\n\tconst X = 1;\n} else {\n\tconst Y = 2;\n}",
            |p| p.eat_if(),
            DestackFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_if_else_breaks_together() {
        let source = r"if (cond1) {
    // comment inside cond1
    const X = 1;
} else {
    const Y = 2;
}";
        assert_format!(
            source,
            source,
            |p| p.eat_if(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_if_else_if_with_comments() {
        let source = r"if (cond1) {
    // comment inside cond1
    const X = 1;
}
// comment before cond2
else if (cond2) {
    const Y = 2; // comment trailing Y
}
// comment before else
else {
    const Z = 3; // comment trailing Z
}";
        assert_format!(
            source,
            source,
            |p| p.eat_if(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_if_else_chain_blank_comment_else_layout() {
        let source = r"if (true) {}

// comment1
else if (false) {}

// comment2

else {}";
        let expected = r"if (true) {
}

// comment1
else if (false) {
}

// comment2
else {
}";
        assert_format!(
            source,
            expected,
            |p| p.eat_if(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_if_empty_blocks_stay_expanded() {
        assert_format!(
            "if (true) {} else {}",
            "if (true) {\n} else {\n}",
            |p| p.eat_if(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_if_let() {
        let source = r"if (const Some(piece) = self.currentPiece) {
    const absolutePositions = piece.getAbsolutePositions(pos);
    for (const blockPos in absolutePositions) {
        if (self.board.isFilled(blockPos)) {
            return true;
        }
    }
}";
        assert_format!(
            source,
            source,
            |p| p.eat_if(),
            DestackFormatOptions::default()
        );
    }
}

mod interface {
    use crate::{DestackFormatOptions, TestFormatter, assert_format};
    use destack_ast::{DeclarationDescriptor, TypeKind};

    #[test]
    fn test_format_interface_empty() {
        assert_format!(
            "interface {}",
            "interface {}",
            |p| p.eat_interface(
                &p.mark(),
                DeclarationDescriptor::default(),
                TypeKind::Structural
            ),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_interface_with_supers() {
        assert_format!(
            "interface Foo extends Bar, Baz {}",
            "interface Foo extends Bar, Baz {}",
            |p| p.eat_interface(
                &p.mark(),
                DeclarationDescriptor::default(),
                TypeKind::Structural
            ),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_interface_with_body() {
        assert_format!(
            "interface Foo { static X = 1 }",
            "interface Foo {\n\tstatic X = 1;\n}",
            |p| p.eat_interface(
                &p.mark(),
                DeclarationDescriptor::default(),
                TypeKind::Structural
            ),
            DestackFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_newtype_interface_empty() {
        assert_format!(
            "interface {}",
            "newtype interface {}",
            |p| p.eat_interface(
                &p.mark(),
                DeclarationDescriptor::default(),
                TypeKind::Nominal
            ),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_newtype_interface_with_method() {
        assert_format!(
            "interface Add<T> { add(other: T): Self }",
            "newtype interface Add<T> {\n\tadd(other: T): Self;\n}",
            |p| p.eat_interface(
                &p.mark(),
                DeclarationDescriptor::default(),
                TypeKind::Nominal
            ),
            DestackFormatOptions::default_tab()
        );
    }
}

mod let_declaration {
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
            "const veryLongIdentifierName =\n\tveryLongIdentifierNameWithManyWords",
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
            r"const shapes = if (self.nextPiece) {
    const nextShape = next.shape;
    self.nextPiece = TetrisPiece.new();
    nextShape
} else {
    self.nextPiece = TetrisPiece.new();
    TetrisGame.getRandomShape(random)
}",
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

mod try_expression {
    use crate::{DestackFormatOptions, TestFormatter, assert_format};

    #[test]
    fn test_format_try_expression() {
        assert_format!(
            "try operation()",
            "try operation()",
            |p| p.eat_try(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_try_block() {
        assert_format!(
            "try { const X = 1 }",
            "try {\n\tconst X = 1;\n}",
            |p| p.eat_try(),
            DestackFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_try_expression_with_catch_match() {
        let source = r"try {
    foo()
} catch match (e) {
    Error(err) => err
}";
        assert_format!(
            source,
            r"try {
    foo();
} catch match (e) {
    Error(err) => err
}",
            |p| p.eat_try(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_try_expression_with_catch_pattern_and_finally() {
        let source = r"try {
    foo()
} catch (e) {
    bar()
} finally {
    baz()
}";
        assert_format!(
            source,
            r"try {
    foo();
} catch (e) {
    bar();
} finally {
    baz();
}",
            |p| p.eat_try(),
            DestackFormatOptions::default()
        );
    }
}
