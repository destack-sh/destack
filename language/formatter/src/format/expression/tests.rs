use crate::{DestackFormatContext, DestackFormatOptions, TestFormatter, assert_format};
use destack_ast::{
    Argument, BinaryOperator, Declaration, DeclarationDescriptor, Expression, LocalNodeId,
    NodeParentIndex, NodeTree, NodeType,
};
use destack_source::{FileType, LanguageType};

/// Build a formatter context for expression classifier assertions.
fn context_from_formatter(formatter: &TestFormatter) -> DestackFormatContext<'_> {
    DestackFormatContext::new(
        DestackFormatOptions::default(),
        &formatter.file,
        &formatter.tree,
        &formatter.tokens,
        &formatter.side_tokens,
        &formatter.side_span,
        &formatter.strings,
        NodeParentIndex::from_tree(&formatter.tree),
    )
}

/// Find the first call expression with the requested dynamic argument count.
fn find_call_with_dynamic_argument_count(
    tree: &NodeTree,
    dynamic_argument_count: usize,
) -> LocalNodeId<Expression> {
    for raw_node_id in 0..tree.next_id() {
        if tree.get_node_type(raw_node_id) != NodeType::Expression {
            continue;
        }

        let expression_id = LocalNodeId::<Expression>::new(raw_node_id);
        let Expression::Call {
            dynamic_arguments, ..
        } = tree.get(expression_id)
        else {
            continue;
        };

        if dynamic_arguments.len() == dynamic_argument_count {
            return expression_id;
        }
    }

    panic!("expected call expression with requested dynamic argument count");
}

/// Find the first parenthesized expression whose inner expression satisfies a predicate.
fn find_parenthesized_expression_by_inner(
    tree: &NodeTree,
    mut predicate: impl FnMut(&Expression) -> bool,
) -> (LocalNodeId<Expression>, LocalNodeId<Expression>) {
    for raw_node_id in 0..tree.next_id() {
        if tree.get_node_type(raw_node_id) != NodeType::Expression {
            continue;
        }

        let expression_id = LocalNodeId::<Expression>::new(raw_node_id);
        let Expression::Parenthesized { expression } = tree.get(expression_id) else {
            continue;
        };
        let inner_expression = tree.get(*expression);
        if predicate(inner_expression) {
            return (expression_id, *expression);
        }
    }

    panic!("expected parenthesized expression matching predicate");
}

/// Simple expressions should stay on one line.
#[test]
fn test_format_expression_simple() {
    assert_format!(
        "1 + 2 * 3 - a / b % c",
        "1 + 2 * 3 - a / b % c",
        |p| p.eat_expression(),
        DestackFormatOptions::default_tab()
    );
}

/// Parenthesized expressions should retain their parentheses.
#[test]
fn test_format_expression_parenthesized() {
    assert_format!(
        "(((1 + 2) * 3) - a / (b % c))",
        "(((1 + 2) * 3) - a / (b % c))",
        |p| p.eat_expression(),
        DestackFormatOptions::default_tab()
    );
}

/// Empty parenthesis/arguments/tuplestuples should be respected.
#[test]
fn test_format_expression_nested_empty_parenthesis() {
    assert_format!(
        "(((())))",
        "(((())))",
        |p| p.eat_expression(),
        DestackFormatOptions::default()
    );
}

/// Empty parenthesis/arguments/tuplestuples should be respected.
#[test]
fn test_format_expression_nested_empty_arguments() {
    assert_format!(
        "foo<()>(((())))",
        "foo<()>(((())))",
        |p| p.eat_expression(),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_expression_struct_literal_trivial() {
    assert_format!(
        "({ a: 1, ...B })",
        "({ a: 1, ...B })",
        |p| p.eat_expression(),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_expression_struct_literal_spread() {
    assert_format!(
        "Foo { ...B }",
        "Foo { ...B }",
        |p| p.eat_expression(),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_assignment_target_detection() {
    let source = "({ className, unfurl: unfurlAttrr, ...attrs } = { className: \"name\", unfurl: \"unfurl\", others: [1, 2, 3] })";
    let (formatter, _expression_id) = TestFormatter::parse(source, |p| p.eat_expression())
        .expect("parse assignment target source");

    let context = context_from_formatter(&formatter);

    let mut found_assignment_target = false;
    for raw_node_id in 0..formatter.tree.next_id() {
        if formatter.tree.get_node_type(raw_node_id) != NodeType::Expression {
            continue;
        }

        let object_expression_id = LocalNodeId::<Expression>::new(raw_node_id);
        let Expression::ObjectExpression { properties, .. } =
            formatter.tree.get(object_expression_id)
        else {
            continue;
        };

        if properties.len() != 3 {
            continue;
        }

        if super::is_assignment_left_target(&context, object_expression_id) {
            found_assignment_target = true;
            break;
        }
    }

    assert!(found_assignment_target);
}

#[test]
fn test_format_expression_if_ternary() {
    assert_format!(
        "true ? 1 : 2",
        "true ? 1 : 2",
        |p| p.eat_expression(),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_expression_index_call_mixed_postfix() {
    assert_format!(
        "x?.[f]?.[2]?.(a, b)",
        "x?.[f]?.[2]?.(a, b)",
        |p| p.eat_expression(),
        DestackFormatOptions::default()
    );
}

/// Instantiation expressions should retain static arguments.
#[test]
fn test_format_expression_instantiation() {
    assert_format!(
        "f<number>",
        "f<number>",
        |p| p.eat_expression(),
        DestackFormatOptions::default()
    );
}

/// New expressions should drop redundant parentheses for simple member callees.
#[test]
fn test_format_new_expression_drops_simple_member_parentheses() {
    assert_format!(
        "new (a.b)()",
        "new a.b()",
        |p| p.eat_expression(),
        DestackFormatOptions::default()
    );
}

/// New expressions should keep optional member callee parentheses.
#[test]
fn test_format_new_expression_keeps_optional_member_parentheses() {
    assert_format!(
        "new (a?.b)()",
        "new (a?.b)()",
        |p| p.eat_expression(),
        DestackFormatOptions::default()
    );
}

/// New expressions should wrap call member callees as a unit.
#[test]
fn test_format_new_expression_wraps_call_member_callee() {
    assert_format!(
        "new (X()).y",
        "new (X().y)()",
        |p| p.eat_expression(),
        DestackFormatOptions::default()
    );
}

/// Parenthesized member objects with boundary comments should not unwrap.
#[test]
fn test_parenthesis_policy_rejects_member_object_boundary_comment() {
    let source = "(value /* boundary */).member";
    let (formatter, _) =
        TestFormatter::parse(source, |p| p.eat_expression()).expect("parse member expression");
    let context = context_from_formatter(&formatter);
    let (parenthesized_id, inner_expression_id) =
        find_parenthesized_expression_by_inner(&formatter.tree, |_| true);

    assert!(!super::parenthesized_should_unwrap(
        &context,
        parenthesized_id,
        inner_expression_id,
        super::ParenthesizedUnwrapPolicy::MemberObject,
    ));
}

/// Parenthesized new callees with optional chains should not unwrap.
#[test]
fn test_parenthesis_policy_rejects_optional_new_callee_unwrap() {
    let source = "new (value?.member)()";
    let (formatter, _) =
        TestFormatter::parse(source, |p| p.eat_expression()).expect("parse new expression");
    let context = context_from_formatter(&formatter);
    let (parenthesized_id, inner_expression_id) =
        find_parenthesized_expression_by_inner(&formatter.tree, |inner_expression| {
            matches!(
                inner_expression,
                Expression::Member { .. } | Expression::PrivateMember { .. }
            )
        });

    assert!(!super::parenthesized_should_unwrap(
        &context,
        parenthesized_id,
        inner_expression_id,
        super::ParenthesizedUnwrapPolicy::NewMemberCallee,
    ));
}

/// Sparse arrays should preserve elision slots.
#[test]
fn test_format_array_expression_sparse_elisions() {
    assert_format!(
        "[,,]",
        "[,,]",
        |p| p.eat_expression(),
        DestackFormatOptions::default()
    );
    assert_format!(
        "[,]",
        "[,]",
        |p| p.eat_expression(),
        DestackFormatOptions::default()
    );
    assert_format!(
        "[1,,]",
        "[1,,]",
        |p| p.eat_expression(),
        DestackFormatOptions::default()
    );
    assert_format!(
        "[1,,3]",
        "[1,, 3]",
        |p| p.eat_expression(),
        DestackFormatOptions::default()
    );
}

/// Empty new arguments should keep boundary comments inside parentheses.
#[test]
fn test_format_new_expression_empty_argument_comment() {
    assert_format!(
        "new require(/* comment */)",
        "new require(/* comment */)",
        |p| p.eat_expression(),
        DestackFormatOptions::default()
    );
}

/// Member expressions should unwrap redundant call object parentheses.
#[test]
fn test_format_member_expression_unwraps_parenthesized_call_object() {
    assert_format!(
        "(require(\"x\")).TraceEntryPointsPlugin",
        "require('x').TraceEntryPointsPlugin",
        |p| p.eat_expression(),
        DestackFormatOptions::default()
    );
}

/// Tree children with map callbacks that return trees force a break.
#[test]
fn test_tree_child_map_callback_breaks() {
    let input = "<List>{items.map((item) => <Item key={item.id} />)}</List>";
    let (formatter, expression_id) =
        TestFormatter::parse(input, |p| p.eat_expression()).expect("parse tree expression");

    // find the tree child expression
    let Expression::TreeExpression { elements, .. } = formatter.tree.get(expression_id) else {
        panic!("expected tree expression");
    };
    let elements = elements.as_ref().expect("expected elements");
    let child_id = elements.first().expect("expected child element");
    let Argument::Positional { value, .. } = formatter.tree.get(*child_id) else {
        panic!("expected positional child");
    };

    // build a context to run the helper on
    let context = context_from_formatter(&formatter);

    assert!(super::expression_has_complex_callback(&context, *value));
}

/// Const on borrows normalizes to readonly in type formatting.
#[test]
fn test_format_type_const_borrow_normalizes_to_readonly() {
    assert_format!("&const Foo", "&readonly Foo", |p| p.eat_expression());
}

/// Const on pointers normalizes to readonly in type formatting.
#[test]
fn test_format_type_const_pointer_normalizes_to_readonly() {
    assert_format!(
        "type T = *const Foo",
        "*readonly Foo",
        |p| p.eat_type(&p.mark(), DeclarationDescriptor::default()),
        |tree: &NodeTree, expr_id| {
            let expression = tree.get(expr_id);
            let Expression::Declaration(declaration_id) = expression else {
                panic!("expected type declaration expression");
            };

            let declaration = tree.get(*declaration_id);
            let Declaration::Type { value, .. } = declaration else {
                panic!("expected type declaration");
            };

            *value
        },
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_member_call_chain_line() {
    assert_format!(
        "call().followed().by().many().calls()",
        "call().followed().by().many().calls()",
        |p| p.eat_expression(),
        DestackFormatOptions::default_tab_with_line_width(100)
    );
}

#[test]
fn test_format_member_call_chain_retains_breaks() {
    assert_format!(
        "call()\n\t.followed()\n\t.by()\n\t.many()\n\t.calls()",
        "call()\n\t.followed()\n\t.by()\n\t.many()\n\t.calls()",
        |p| p.eat_expression(),
        DestackFormatOptions::default_tab_with_line_width(20)
    );
}

#[test]
fn test_format_member_call_chain_breaks() {
    assert_format!(
        "call().followed().by().many().calls()",
        "call()\n\t.followed()\n\t.by()\n\t.many()\n\t.calls()",
        |p| p.eat_expression(),
        DestackFormatOptions::default_tab_with_line_width(20)
    );
}

#[test]
fn test_format_member_call_chain_breaks_with_maybe_and_index() {
    assert_format!(
        "call().followed()?.by()[0]?.many()?.calls()",
        "call()\n\t.followed()\n\t?.by()\n\t[0]\n\t?.many()\n\t?.calls()",
        |p| p.eat_expression(),
        DestackFormatOptions::default_tab_with_line_width(20)
    );
}

#[test]
fn test_format_path_member_call_chain_breaks() {
    assert_format!(
        "long.base.path.followed().by().many().calls()",
        "long.base\n\t.path\n\t.followed()\n\t.by()\n\t.many()\n\t.calls()",
        |p| p.eat_expression(),
        DestackFormatOptions::default_tab_with_line_width(20)
    );
}

#[test]
fn test_format_index_member_chain_breaks() {
    assert_format!(
        "identifier1.identifier2.identifier3[indexA].identifier4[indexB]?.[indexC][indexD]",
        "identifier1\n\t.identifier2\n\t.identifier3[indexA]\n\t.identifier4[indexB]\n\t?.[indexC]\n\t[indexD]",
        |p| p.eat_expression(),
        DestackFormatOptions::default_tab_with_line_width(20)
    );
}

/// Member chains break before long boundary comments when optional calls follow.
#[test]
fn test_format_member_chain_breaks_before_long_boundary_comment_with_optional_call() {
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    assert_format!(
        "this.getParameters /* xxxxxxxxxxxxxxxxxxxxxxxxxxxx */\n  ?.();",
        "this\n  .getParameters /* xxxxxxxxxxxxxxxxxxxxxxxxxxxx */\n  ?.()",
        |p| p.eat_expression(),
        options
    );
}

/// Chain planner keeps a short promoted head for call-like argument chains.
#[test]
fn test_format_chain_planner_promotes_head_in_call_like_argument() {
    assert_format!(
        "render(foo.bar.getResource(id).map(transform).finalize())",
        "render(foo.bar.getResource(id)\n    .map(transform)\n    .finalize(),)",
        |p| p.eat_expression(),
        DestackFormatOptions::default_with_line_width(30)
    );
}

/// Chain planner uses assignment rhs width when choosing chain layout.
#[test]
fn test_format_chain_planner_respects_assignment_rhs_width() {
    assert_format!(
        "veryLongBindingName = source.alpha.beta.gamma().delta().epsilon()",
        "veryLongBindingName = source.alpha.beta\n    .gamma()\n    .delta()\n    .epsilon()",
        |p| p.eat_expression(),
        DestackFormatOptions::default_with_line_width(40)
    );
}

#[test]
fn test_format_expression_tree_literal_without_arguments() {
    assert_format!(
        "<Entity/>",
        "<Entity />",
        |p| p.eat_expression(),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_expression_tree_literal_with_arguments() {
    assert_format!(
        "<Entity a={1} b = {2} />",
        "<Entity a={1} b={2} />",
        |p| p.eat_expression(),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_expression_tree_literal_parenthesized() {
    let source = r"(
    <Entity a={1} b={2}>
        <Entity a={1} b={2} />
    </Entity>
)";
    let expected = r"(
    <Entity a={1} b={2}>
        <Entity a={1} b={2} />
    </Entity>
)";
    assert_format!(
        source,
        expected,
        |p| p.eat_expression(),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_expression_tree_literal_nested() {
    let source = r#"<A x={4} y={4}>
    <B x="hey">
        <C>
            <D />
            2
        </C>
    </B>
</A>"#;
    let expected = r#"(
    <A x={4} y={4}>
        <B x="hey">
            <C>
                <D />
                2
            </C>
        </B>
    </A>
)"#;
    assert_format!(
        source,
        expected,
        |p| p.eat_expression(),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_expression_tree_literal_with_array_of_struct_element() {
    // multiline array attributes break the element
    let source = r#"<Menu
    items={[
        { to: "/posts" },
        { to: "/posts/$postId", params: { postId: "postId" } },
    ]}
/>"#;
    let expected = r#"(
    <Menu
        items={[
            { to: "/posts" },
            { to: "/posts/$postId", params: { postId: "postId" } },
        ]}
    />
)"#;
    assert_format!(
        source,
        expected,
        |p| p.eat_expression(),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_expression_call_with_struct_literal() {
    let source = r#"Destack.serve({
    fetch: function (req: Request) {
        return Response("Success!")
    },
    run: true,
})"#;
    assert_format!(
        source,
        r#"Destack.serve({
    fetch: function (req: Request) {
        return Response("Success!");
    },
    run: true,
})"#,
        |p| p.eat_expression(),
        DestackFormatOptions::default_with_line_width(40)
    );
}

#[test]
fn test_format_expression_let_call() {
    let input = r#"const ast = await parseAsync(text, {
    sourceFileName: "file",
    parserOpts: {
        plugins: ["typescript", "jsx"],
    },
    sourceType: "module",
    configFile: false,
    babelrc: false,
})"#;
    let expected = r#"const ast = await parseAsync(
    text,
    {
        sourceFileName: "file",
        parserOpts: {
            plugins: [
                "typescript",
                "jsx",
            ],
        },
        sourceType: "module",
        configFile: false,
        babelrc: false,
    },
)"#;
    assert_format!(
        input,
        expected,
        |p| p.eat_expression(),
        DestackFormatOptions::default_with_line_width(40)
    );
}

#[test]
fn test_format_call_single_lambda_argument_with_prefix_comment_breaks() {
    let source = "call(/* comment */\n  () => {\n    //\n  }\n)";
    let expected = "call(\n    /* comment */\n    () => {\n        //\n    },\n)";
    assert_format!(source, expected, |p| p.eat_expression());
}

#[test]
fn test_format_call_nested_arrow_boundary_comments() {
    let source = "call(\n  () /**/ => //\n    () /**/ => /**/\n      () /**/ => /**/ {\n        //\n      }\n)";
    let expected =
        "call(() /**/ =>\n    //\n    () /**/ =>\n    /**/\n    () /**/ => /**/ {\n    //\n})";
    assert_format!(source, expected, |p| p.eat_expression());
}

#[test]
fn test_format_chained_assignment() {
    assert_format!(
        "a = b = c = 1",
        "a = b = c = 1",
        |p| p.eat_expression(),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_chained_assignment_long() {
    assert_format!(
        "veryLongName = anotherLongName = thirdLongName = 42",
        "veryLongName =\n    anotherLongName =\n    thirdLongName =\n    42",
        |p| p.eat_expression(),
        DestackFormatOptions::default_with_line_width(30)
    );
}

#[test]
fn test_format_jsx_with_comment() {
    assert_format!(
        "<Container>{/* XOXO */}</Container>",
        "(\n    <Container>\n        {/* XOXO */}\n    </Container>\n)",
        |p| p.eat_expression(),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_jsx_conditional_child() {
    assert_format!(
        "<div>{loading && <Spinner />}</div>",
        "<div>{loading && <Spinner />}</div>",
        |p| p.eat_expression(),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_jsx_in_function_call() {
    assert_format!(
        "render(<App />)",
        "render(<App />)",
        |p| p.eat_expression(),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_deeply_nested_callbacks() {
    assert_format!(
        "fetch(url).then((res) => res.json()).then((data) => process(data))",
        "fetch(url)\n    .then((res) => res.json())\n    .then((data) => process(data))",
        |p| p.eat_expression(),
        DestackFormatOptions::default_with_line_width(40)
    );
}

/// Callback-heavy chain calls force expanded argument formatting.
#[test]
fn test_call_chain_classifier_expands_callback_heavy_arguments() {
    let source =
        "compose((value) => step1(value), (value) => step2(value), (value) => step3(value)).run()";
    let (formatter, _expression_id) =
        TestFormatter::parse(source, |p| p.eat_expression()).expect("parse callback-heavy call");
    let context = context_from_formatter(&formatter);

    let call_id = find_call_with_dynamic_argument_count(&formatter.tree, 3);
    let Expression::Call {
        dynamic_arguments, ..
    } = formatter.tree.get(call_id)
    else {
        panic!("expected callback-heavy call expression");
    };

    assert!(super::call_arguments_force_expand_for_chain(
        &context,
        call_id,
        dynamic_arguments
    ));
}

/// Single JSX or tree child arguments in chains force expansion.
#[test]
fn test_call_chain_classifier_expands_single_tree_child_argument() {
    let source = "render(<App><Body /></App>).run()";
    let (formatter, _expression_id) =
        TestFormatter::parse(source, |p| p.eat_expression()).expect("parse tree-argument call");
    let context = context_from_formatter(&formatter);

    let call_id = find_call_with_dynamic_argument_count(&formatter.tree, 1);
    let Expression::Call {
        dynamic_arguments, ..
    } = formatter.tree.get(call_id)
    else {
        panic!("expected tree-argument call expression");
    };

    assert!(super::call_arguments_force_expand_for_chain(
        &context,
        call_id,
        dynamic_arguments
    ));
}

#[test]
fn test_format_optional_chain_with_nullish() {
    assert_format!(
        r#"user?.profile?.name ?? "Anonymous""#,
        r#"user?.profile?.name ?? "Anonymous""#,
        |p| p.eat_expression(),
        DestackFormatOptions::default()
    );
}

/// Formats type conditionals with infer bindings.
#[test]
fn test_format_type_conditional_with_infer() {
    assert_format!(
        "type Result = T extends infer U ? U : never",
        "type Result = T extends infer U ? U : never;",
        |p| p.eat_expression()
    );
}

/// Formats type conditionals with constrained infer bindings.
#[test]
fn test_format_type_conditional_with_constrained_infer() {
    assert_format!(
        "type Result = T extends infer U extends string ? U : never",
        "type Result = T extends infer U extends string ? U : never;",
        |p| p.eat_expression()
    );
}

/// Formats nested conditional types with explicit parentheses.
#[test]
fn test_format_type_conditional_nested() {
    assert_format!(
        "type Result = T extends U ? (U extends V ? X : Y) : Z",
        "type Result = T extends U ? (U extends V ? X : Y) : Z;",
        |p| p.eat_expression()
    );
}

/// Formats type intersections with trailing operators in Destack.
#[test]
fn test_format_type_intersection_trailing_operator_destack() {
    assert_format!(
        "type Combined = HasName & HasAge & HasEmail",
        "type Combined = HasName &\n    HasAge &\n    HasEmail;",
        |p| p.eat_expression(),
        DestackFormatOptions::default_with_line_width(30)
    );
}

/// Formats mapped types with modifiers and key remaps.
#[test]
fn test_format_type_mapped_with_remap() {
    let source = r#"type Remap = { readonly [K in keyof T as `${K}`]-?: T[K] }"#;
    let expected = r#"type Remap = { readonly [K in keyof T as `${K}`]-?: T[K] };"#;
    assert_format!(source, expected, |p| p.eat_expression());
}

/// Formats mapped types with removal modifiers.
#[test]
fn test_format_type_mapped_with_removals() {
    let source = r#"type Mutable = { -readonly [K in keyof T]-?: T[K] }"#;
    let expected = r#"type Mutable = { -readonly [K in keyof T]-?: T[K] };"#;
    assert_format!(source, expected, |p| p.eat_expression());
}

/// Formats mapped types without modifiers.
#[test]
fn test_format_type_mapped_without_modifiers() {
    let source = r#"type Plain = { [K in keyof T]: T[K] }"#;
    let expected = r#"type Plain = { [K in keyof T]: T[K] };"#;
    assert_format!(source, expected, |p| p.eat_expression());
}

/// Formats mapped types with optional modifiers.
#[test]
fn test_format_type_mapped_with_optional() {
    let source = r#"type Optional = { [K in keyof T]?: T[K] }"#;
    let expected = r#"type Optional = { [K in keyof T]?: T[K] };"#;
    assert_format!(source, expected, |p| p.eat_expression());
}

/// Formats chained type index expressions.
#[test]
fn test_format_type_index() {
    let source = r#"type Value = T[K][P]"#;
    let expected = r#"type Value = T[K][P];"#;
    assert_format!(source, expected, |p| p.eat_expression());
}

/// Formats type template literals with single spans.
#[test]
fn test_format_type_template_literal() {
    let source = r#"type Key = `on${K}`"#;
    let expected = r#"type Key = `on${K}`;"#;
    assert_format!(source, expected, |p| p.eat_expression());
}

/// Formats type template literals with multiple spans.
#[test]
fn test_format_type_template_literal_multiple_spans() {
    let source = r#"type Key = `on${K}:${V}`"#;
    let expected = r#"type Key = `on${K}:${V}`;"#;
    assert_format!(source, expected, |p| p.eat_expression());
}

/// Formats template literal type unions with leading `|` style.
#[test]
fn test_format_type_template_literal_union_with_leading_pipe() {
    let source = "type T = `${\n  | 'W'\n  | 'I'\n  | 'L'\n  | 'L'\n  | 'B'\n  | 'R'\n  | 'E'\n  | 'A'\n  | 'K'\n}${'!' | '!!'}`";
    let expected = "type T = `${\n    | 'W'\n    | 'I'\n    | 'L'\n    | 'L'\n    | 'B'\n    | 'R'\n    | 'E'\n    | 'A'\n    | 'K'}${'!' | \"!!\"}`;";
    assert_format!(source, expected, |p| p.eat_expression());
}

/// Drops redundant wrappers around associative type unions.
#[test]
fn test_format_type_union_drops_redundant_parentheses() {
    let source = "type C = | (| (| A | B))";
    let expected = "type C = \n    | A\n    | B;";
    assert_format!(source, expected, |p| p.eat_expression());
}

/// Drops single-member leading union wrappers in parenthesized array element types.
#[test]
fn test_format_type_single_member_leading_union_parenthesized_array() {
    let source = "type Items = ( | number)[]";
    let expected = "type Items = number[];";
    assert_format!(source, expected, |p| p.eat_expression());
}

/// Drops single-member leading intersection wrappers in parenthesized array element types.
#[test]
fn test_format_type_single_member_leading_intersection_parenthesized_array() {
    let source = "type Items = ( & number)[]";
    let expected = "type Items = number[];";
    let (test, expression_id) =
        TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| p.eat_expression())
            .expect("parse typescript expression");
    let mut options = DestackFormatOptions::default();
    options.language_type = LanguageType::TypeScript;
    let formatted = test.format(&expression_id, options);
    assert_eq!(formatted, expected);
}

/// Type template literal unions are tracked in type context.
#[test]
fn test_type_template_literal_union_is_in_type_context() {
    let source = "type T = `${\n  | 'W'\n  | 'I'\n}${'!' | '!!'}`";
    let (formatter, expression_id) =
        TestFormatter::parse(source, |p| p.eat_expression()).expect("parse template literal type");

    let context = context_from_formatter(&formatter);

    let mut has_union = false;
    let mut has_union_in_type_context = false;
    let mut has_union_with_leading_pipe_source = false;
    for raw_node_id in 0..formatter.tree.next_id() {
        if formatter.tree.get_node_type(raw_node_id) != NodeType::Expression {
            continue;
        }

        let expression_id = LocalNodeId::<Expression>::new(raw_node_id);
        let Expression::Binary { operator, .. } = formatter.tree.get(expression_id) else {
            continue;
        };
        if *operator != BinaryOperator::ElementwiseOr {
            continue;
        }

        has_union = true;
        if super::union_source_has_leading_pipe(&context, expression_id) {
            has_union_with_leading_pipe_source = true;
        }
        if super::is_type_context(&context, expression_id) {
            has_union_in_type_context = true;
        }
    }

    let _ = expression_id;
    assert!(has_union);
    assert!(has_union_in_type_context);
    assert!(has_union_with_leading_pipe_source);
}

/// Formats type imports with qualifiers.
#[test]
fn test_format_type_import() {
    assert_format!(
        r#"type Imported = import("mod").Type"#,
        r#"type Imported = import("mod").Type;"#,
        |p| p.eat_expression()
    );
}

/// Formats type imports without qualifiers.
#[test]
fn test_format_type_import_without_qualifier() {
    assert_format!(
        r#"type Imported = import("mod")"#,
        r#"type Imported = import("mod");"#,
        |p| p.eat_expression()
    );
}

/// Formats standalone infer expressions.
#[test]
fn test_format_type_infer_expression() {
    assert_format!("type Result = infer U", "type Result = infer U;", |p| p
        .eat_expression());
}

#[test]
fn test_format_async_arrow() {
    assert_format!(
        "async (event) => await processEvent(event)",
        "async (event) => await processEvent(event)",
        |p| p.eat_expression(),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_return_jsx_inline() {
    // short JSX returns stay inline
    assert_format!(
        "return <App />",
        "return <App />",
        |p| p.eat_expression(),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_return_jsx_multiline() {
    assert_format!(
        "return <App prop=\"value\" another=\"thing\" />",
        "return (\n    <App\n        prop=\"value\"\n        another=\"thing\"\n    />\n)",
        |p| p.eat_expression(),
        DestackFormatOptions::default_with_line_width(30)
    );
}

#[test]
fn test_format_nested_ternary() {
    assert_format!(
        "const x = isFirst ? firstValue : isSecond ? secondValue : defaultValue",
        "const x = isFirst\n    ? firstValue\n    : isSecond\n    ? secondValue\n    : defaultValue",
        |p| p.eat_expression(),
        DestackFormatOptions::default_with_line_width(50)
    );
}

/// Keeps call rhs attached to `=` while call arguments break internally.
#[test]
fn test_format_const_call_with_multiline_object_rhs() {
    assert_format!(
        "const server = createServer({ port: config.server.port, middleware: [corsMiddleware(config.server.cors), authMiddleware(), loggingMiddleware({ level: \"info\" })], routes: [userRouter] })",
        "const server = createServer({\n    port: config.server.port,\n    middleware: [\n        corsMiddleware(config.server.cors),\n        authMiddleware(),\n        loggingMiddleware({ level: \"info\" }),\n    ],\n    routes: [userRouter],\n})",
        |p| p.eat_expression(),
        DestackFormatOptions::default_with_line_width(70)
    );
}

/// Keeps chain rhs attached to `=` while chain operations break internally.
#[test]
fn test_format_export_const_chain_rhs_does_not_break_after_operator() {
    assert_format!(
        "export const userRouter =\n    Router\n    .create()\n    .get(\"/users\", async (ctx) => {\n        return Response.json(ctx);\n    })\n    .post(\"/users\", async (ctx) => {\n        return Response.created(ctx);\n    })",
        "export const userRouter = Router.create()\n    .get(\"/users\", async (ctx) => {\n        return Response.json(ctx);\n    })\n    .post(\"/users\", async (ctx) => {\n        return Response.created(ctx);\n    })",
        |p| p.eat_expression(),
        DestackFormatOptions::default_with_line_width(80)
    );
}

/// Breaks after `=` for long generic call rhs values.
#[test]
fn test_format_const_generic_call_rhs_breaks_after_operator() {
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    assert_format!(
        "const result = configurationService.getValue<Record<string, boolean>>(enalementSetting)",
        "const result =\n  configurationService.getValue<Record<string, boolean>>(enalementSetting)",
        |p| p.eat_expression(),
        options
    );
}

/// Preserves break-after-operator for multiline generic call rhs values.
#[test]
fn test_format_const_generic_call_rhs_preserves_source_operator_break() {
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    assert_format!(
        "const result =\n  configurationService.getValue<Record<string, boolean>>(\n  enalementSetting\n)",
        "const result =\n  configurationService.getValue<Record<string, boolean>>(enalementSetting)",
        |p| p.eat_expression(),
        options
    );
}

/// Keeps `=` inline for multiline object-like generic call rhs values.
#[test]
fn test_format_const_generic_call_with_multiline_type_argument_keeps_operator_inline() {
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    assert_format!(
        "const emitter = createGlobalEmitter<{\n  key: Extract<Event, { type: key }>\n}>()",
        "const emitter = createGlobalEmitter<{\n  key: Extract<Event, { type: key }>,\n}>()",
        |p| p.eat_expression(),
        options
    );
}

#[test]
fn test_format_jsx_bracket_same_line_true() {
    let mut options = DestackFormatOptions::default_with_line_width(30);
    options.bracket_same_line = true;
    assert_format!(
        r#"<Button variant="primary" size="large" disabled />"#,
        "(\n    <Button\n        variant=\"primary\"\n        size=\"large\"\n        disabled />\n)",
        |p| p.eat_expression(),
        options
    );
}

#[test]
fn test_format_jsx_bracket_same_line_false() {
    let mut options = DestackFormatOptions::default_with_line_width(30);
    options.bracket_same_line = false;
    assert_format!(
        r#"<Button variant="primary" size="large" disabled />"#,
        "(\n    <Button\n        variant=\"primary\"\n        size=\"large\"\n        disabled\n    />\n)",
        |p| p.eat_expression(),
        options
    );
}

/// Prefix expressions inside postfix operators get parenthesized.
#[test]
fn test_format_await_inside_maybe_gets_parenthesized() {
    // this tests the case where we have Maybe { left: Await { expr } }
    // which should format as (await expr)? not await expr?
    assert_format!(
        "(await foo())?",
        "(await foo())?",
        |p| p.eat_expression(),
        DestackFormatOptions::default()
    );
}

/// Unary prefix inside maybe gets parenthesized.
#[test]
fn test_format_unary_inside_maybe_gets_parenthesized() {
    assert_format!(
        "(-x)?",
        "(-x)?",
        |p| p.eat_expression(),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_unary_await_expression_parenthesizes_operand() {
    assert_format!(
        "async () => !await foo()",
        "async () => !(await foo())",
        |p| p.eat_expression(),
        DestackFormatOptions::default()
    );
}

/// Postfix inside postfix doesn't need extra parentheses.
#[test]
fn test_format_postfix_inside_maybe_no_extra_parens() {
    assert_format!(
        "x.foo?",
        "x.foo?",
        |p| p.eat_expression(),
        DestackFormatOptions::default()
    );
}

/// Call expression inside maybe doesn't need parentheses.
#[test]
fn test_format_call_inside_maybe_no_parens() {
    assert_format!(
        "foo()?",
        "foo()?",
        |p| p.eat_expression(),
        DestackFormatOptions::default()
    );
}

/// Await? syntactic sugar stays as is.
#[test]
fn test_format_await_maybe_sugar() {
    assert_format!(
        "await? foo()",
        "await? foo()",
        |p| p.eat_expression(),
        DestackFormatOptions::default()
    );
}
