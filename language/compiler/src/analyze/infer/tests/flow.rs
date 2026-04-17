use super::*;
use crate::run_to_completion;
use destack_dir::NormalizationMode;

/// Preserve function body and branch value tails from declared into analyzed DIR.
#[test]
fn test_preserve_function_if_block_value_tails_into_analyze() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function choose(flag: boolean, a: int32, b: int32): int32 {
    if (flag) { a } else { b }
}
"#,
    );

    // check the parsed ast shape first
    run_to_completion(
        &test.compiler,
        test.program.current_revision(),
        |compiler, context| compiler.process_ast(module_id, context),
    )
    .unwrap_or_else(|error| panic!("failed to parse module {module_id:?}: {error:?}"));
    let ast = test
        .repository
        .ast(test.program.current_revision(), module_id)
        .expect("expected ast artifact");
    assert_choose_body_if_tails_in_ast_tree(&ast.tree, &ast.roots);

    // check the imported artifacts before analysis
    test.import_module(module_id);
    test.compile();
    let base = test.dir_base(module_id);
    assert_choose_body_if_tails_in_tree(base.tree.as_ref(), base.roots.as_ref());

    test.resolve_module(module_id);
    test.compile();
    let resolved = test.dir_resolved(module_id);
    assert_choose_body_if_tails_in_tree(resolved.tree.as_ref(), resolved.roots.as_ref());

    // check the declared tree shape next
    test.declare_module(module_id);
    let declared_view = test.declared_view(module_id);
    assert_choose_body_if_tails(&declared_view);

    // drive analysis without requiring clean diagnostics
    test.analyze_module(module_id);
    test.compile();
    let analyzed_view = test.view(module_id);
    assert_choose_body_if_tails(&analyzed_view);
}

// function body and branch tails
fn assert_choose_body_if_tails(view: &TestModuleView<'_>) {
    assert_choose_body_if_tails_in_tree(view.tree(), view.roots());
}

// function body and branch tails in one ast tree
fn assert_choose_body_if_tails_in_ast_tree(
    tree: &destack_ast::NodeTree,
    roots: &[destack_ast::LocalNodeId<destack_ast::Expression>],
) {
    let root_id = roots
        .first()
        .copied()
        .expect("expected one root expression");
    let destack_ast::Expression::Declaration(declaration_id) = tree.get(root_id) else {
        panic!("expected function declaration root");
    };
    let destack_ast::Declaration::Function(declaration) = tree.get(*declaration_id) else {
        panic!("expected function body");
    };
    let body_id = declaration.body.expect("expected function body");
    let destack_ast::Expression::Block(block_id) = tree.get(body_id) else {
        panic!("expected function body block");
    };
    let block = tree.get(*block_id);

    assert!(block.leading_expressions.is_empty());

    let tail_expression_id = block.tail_expression.expect("expected function body tail");
    let destack_ast::Expression::If {
        then_expression,
        else_expression: Some(else_expression),
        ..
    } = tree.get(tail_expression_id)
    else {
        panic!("expected tail if expression");
    };

    let destack_ast::Expression::Block(then_block_id) = tree.get(*then_expression) else {
        panic!("expected then block");
    };
    let then_block = tree.get(*then_block_id);
    assert!(then_block.leading_expressions.is_empty());
    assert!(then_block.tail_expression.is_some());

    let destack_ast::Expression::Block(else_block_id) = tree.get(*else_expression) else {
        panic!("expected else block");
    };
    let else_block = tree.get(*else_block_id);
    assert!(else_block.leading_expressions.is_empty());
    assert!(else_block.tail_expression.is_some());
}

// function body and branch tails in one dir tree
fn assert_choose_body_if_tails_in_tree(tree: &NodeTree, roots: &[LocalNodeId<Expression>]) {
    let root_id = root_expression_id(roots, tree, 0);
    let Expression::Declaration(declaration) = tree.get(root_id) else {
        panic!("expected function declaration root");
    };
    let Declaration::Function(declaration) = tree.get(*declaration) else {
        panic!("expected function body");
    };
    let body_id = declaration.body.expect("expected function body");
    let Expression::Block(block) = tree.get(body_id) else {
        panic!("expected function body block");
    };
    let block = tree.get(*block);

    assert!(block.leading_expressions.is_empty());

    let tail_expression_id = block.tail_expression.expect("expected function body tail");
    let Expression::If {
        then_expression,
        else_expression: Some(else_expression),
        ..
    } = tree.get(tail_expression_id)
    else {
        panic!("expected tail if expression");
    };

    let Expression::Block(then_block_id) = tree.get(*then_expression) else {
        panic!("expected then block");
    };
    let then_block = tree.get(*then_block_id);
    assert!(then_block.leading_expressions.is_empty());
    assert!(then_block.tail_expression.is_some());

    let Expression::Block(else_block_id) = tree.get(*else_expression) else {
        panic!("expected else block");
    };
    let else_block = tree.get(*else_block_id);
    assert!(else_block.leading_expressions.is_empty());
    assert!(else_block.tail_expression.is_some());
}

/// Build a flow graph with true and false branches for if expressions.
#[test]
fn test_build_flow_graph_if_expression() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        "const value = true; if (value) { 1 } else { 2 };",
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load tree data
    let view = test.view(module_id);

    // locate the if expression
    let if_expression_id = view
        .roots()
        .iter()
        .find_map(|root_id| match view.tree().get(*root_id) {
            Expression::If { .. } => Some(*root_id),
            _ => None,
        })
        .expect("expected if expression");

    // build the flow graph
    let graph = FlowGraphBuilder::new(module_id, view.tree()).build(if_expression_id);

    let mut has_true_edge = false;
    let mut has_false_edge = false;
    let mut has_join_block = false;
    for block in &graph.blocks {
        if block.predecessors.len() >= 2 {
            has_join_block = true;
        }
        for edge in &block.successors {
            if edge.kind == FlowEdgeKind::True {
                has_true_edge = true;
            }
            if edge.kind == FlowEdgeKind::False {
                has_false_edge = true;
            }
        }
    }

    assert!(has_true_edge);
    assert!(has_false_edge);
    assert!(has_join_block);
}

/// Build a flow graph that narrows the right side of short circuit guards.
#[test]
fn test_build_flow_graph_short_circuit_guard() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
        const value: string | null = null;
        const accepts_string = (input: string): boolean => true;
        if (value !== null && accepts_string(value)) { };
        "#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load tree data
    let module = test.program.module_descriptor(module_id);
    let module = module.as_ref();
    let profile = test.default_profile_id(module_id);
    let dir = test.artifact_dir(module_id, profile);
    let mut dir = dir;
    let roots = dir.roots.clone();
    let tree = &mut dir.tree;
    let symbols = &mut dir.symbols;
    let types = &mut dir.types;

    // locate the if expression
    let if_expression_id = roots
        .iter()
        .find_map(|root_id| match tree.get(*root_id) {
            Expression::If { .. } => Some(*root_id),
            _ => None,
        })
        .expect("expected if expression");

    // locate the right side of the condition
    let Expression::If { condition, .. } = tree.get(if_expression_id) else {
        panic!("expected if expression");
    };
    let condition_id = match condition {
        IfCondition::Expression { condition } => *condition,
        IfCondition::Let { .. } => panic!("expected expression condition"),
    };
    let Expression::Binary {
        left,
        operator,
        right,
    } = tree.get(condition_id)
    else {
        panic!("expected binary condition");
    };
    assert_eq!(*operator, BinaryOperator::And);

    let Expression::Binary { left, .. } = tree.get(*left) else {
        panic!("expected binary left guard");
    };

    let Expression::Call { arguments, .. } = tree.get(*right) else {
        panic!("expected call expression");
    };
    let argument_id = arguments.first().copied().expect("expected call argument");
    let argument = tree.get(argument_id);
    let argument_value_id = argument.value();

    // ensure the right side reference uses a distinct node id
    assert_ne!(*left, argument_value_id);

    // build flow data for the condition expression
    // build the flow graph
    let graph = FlowGraphBuilder::new(module.id, tree).build(condition_id);
    let context = InferState::new(profile, AnalyzeOptions::from(&CompilerOptions::default()));
    let compiler_context = test.context();
    let mut flow_ctx = TypeContext::new(
        &compiler_context,
        module,
        profile,
        &context.options,
        tree,
        symbols,
        types,
        AnalyzeIndex::default(),
    );
    let flow = test
        .compiler
        .compute_flow_table_for_graph(&mut flow_ctx, &graph, &context)
        .expect("expected flow table");

    // read the flow environment for the right side argument
    let block_id = graph
        .block_by_node
        .get(&argument_value_id.into_any())
        .copied()
        .expect("expected block for argument");
    let environment_id = flow
        .entry_environment_for_block(block_id)
        .expect("expected entry environment");
    let environment = flow
        .environment(environment_id)
        .expect("expected environment");

    // assert that the value symbol is narrowed to a non null type
    let value_symbol = test
        .resolve_to_symbol("test.ds", "value")
        .expect("expected value symbol");
    let narrowed_type_id = environment
        .bindings
        .get(&value_symbol)
        .copied()
        .expect("expected narrowing for value");
    let is_null = match types.get_type(narrowed_type_id) {
        Type::TypeLiteral {
            value: TypeLiteral::Null,
        } => true,
        Type::Union { elements } => elements.iter().any(|element_id| {
            matches!(
                types.get_type(*element_id),
                Type::TypeLiteral {
                    value: TypeLiteral::Null
                }
            )
        }),
        _ => false,
    };
    assert!(!is_null);
}

/// Build a flow graph with a back edge for for loops.
#[test]
fn test_build_flow_graph_for_loop() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
        function run() {
            for (let i: number = 0; i < 3; i = i + 1) {
                i = i + 1;
            }
        }
        "#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load tree data
    let view = test.view(module_id);

    // locate the first function body
    let function_body_id = view
        .roots()
        .iter()
        .find_map(|root_id| match view.tree().get(*root_id) {
            Expression::Declaration(declaration) => match view.tree().get(*declaration) {
                Declaration::Function(declaration) => declaration.body,
                _ => None,
            },
            _ => None,
        })
        .expect("expected function body");

    // build the flow graph
    let graph = FlowGraphBuilder::new(module_id, view.tree()).build(function_body_id);

    let mut has_true_edge = false;
    let mut has_false_edge = false;
    let mut has_back_edge = false;
    for block in &graph.blocks {
        for edge in &block.successors {
            if edge.kind == FlowEdgeKind::True {
                has_true_edge = true;
            }
            if edge.kind == FlowEdgeKind::False {
                has_false_edge = true;
            }
            if edge.target.0 < block.id.0 {
                has_back_edge = true;
            }
        }
    }

    assert!(has_true_edge);
    assert!(has_false_edge);
    assert!(has_back_edge);
}

/// Analyze flow heavy helpers without stalling on loop back edge merges.
#[test]
fn test_analyze_flow_helpers_converge_for_loop_and_union_narrowing() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ts",
        r#"
        function normalizeUri(uri: string): string {
            const trimmedUri = uri.split("?")[0];

            return trimmedUri.replace(/\/+$/, "");
        }

        function parseCookies(header: string) {
            const cookies = {};

            for (const part of header.split(";")) {
                const segments = part.split("=");
                const key = segments[0];

                if (key === undefined) {
                    continue;
                }

                const value = segments.slice(1).join("=").trim();
                cookies[key.trim()] = value;
            }

            return cookies;
        }

        function matchesRedirectUri(incoming: string, registered: string[]) {
            const normalizedIncoming = normalizeUri(incoming);
            let index = 0;

            while (index < registered.length) {
                const registeredUri = registered[index];

                if (normalizeUri(registeredUri) === normalizedIncoming) {
                    return true;
                }

                index = index + 1;
            }

            return false;
        }

        function bodyStr(value: string | string[]) {
            if (typeof value === "string") {
                return value;
            }

            const bodyValue = value[0];

            if (bodyValue === undefined) {
                return "";
            }

            return bodyValue;
        }

        const cookies = parseCookies("session=abc123; theme=dark");
        const requestBody = bodyStr(["draft"]);
        const redirectAllowed = matchesRedirectUri(
            "https://destack.dev/docs/?q=1",
            ["https://destack.dev/docs", "https://destack.dev/app"],
        );
        "#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let request_body_symbol = test.resolve_to_symbol("test.ts", "requestBody").unwrap();
    let redirect_allowed_symbol = test
        .resolve_to_symbol("test.ts", "redirectAllowed")
        .unwrap();

    test.with_dir_types_mut(module_id, |module, profile, _dir, tree, symbols, types| {
        let options = test.analyze_context_options_for_module(module.id);
        let compiler_context = test.context();
        let mut ctx = TypeContext::new(
            &compiler_context,
            module,
            profile,
            &options,
            tree,
            symbols,
            types,
            AnalyzeIndex::default(),
        );

        // request body normalizes back to string after union narrowing
        let request_body_type_id = ctx.types.get_value_type_id(request_body_symbol).unwrap();
        let request_body_type_id = test.compiler.normalize_type(
            &mut ctx.reborrow(),
            request_body_type_id,
            NormalizationMode::Assign,
        );
        assert_eq!(
            *ctx.types.get_type(request_body_type_id),
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::String)
            }
        );

        // redirect result stays boolean after loop convergence
        let redirect_allowed_type_id = ctx
            .types
            .get_value_type_id(redirect_allowed_symbol)
            .unwrap();
        let redirect_allowed_type_id = test.compiler.normalize_type(
            &mut ctx.reborrow(),
            redirect_allowed_type_id,
            NormalizationMode::Assign,
        );
        assert_eq!(
            *ctx.types.get_type(redirect_allowed_type_id),
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Boolean)
            }
        );
    });
}

/// Collapse string literal returns under a string return family.
#[test]
fn test_analyze_return_convergence_drops_string_literal_under_string() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ts",
        r#"
        function collapse(value: string, flag: boolean) {
            if (flag) {
                return value;
            }

            return "";
        }

        const collapsed = collapse("draft", true);
        "#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);
    let collapsed_symbol = test.resolve_to_symbol("test.ts", "collapsed").unwrap();

    // collapse the literal branch under string
    let collapsed_type = view.types().get_value_type(collapsed_symbol).unwrap();
    assert_eq!(
        *collapsed_type,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String)
        }
    );
}

/// Preserve computed member assignment shapes through analyzed DIR.
#[test]
fn test_analyze_preserves_computed_member_assignment_shape() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ts",
        r#"
        function parseCookies(header: string) {
            const cookies = {};

            for (const part of header.split(";")) {
                const segments = part.split("=");
                const key = segments[0];
                const value = segments.slice(1).join("=").trim();

                cookies[key.trim()] = value;
            }

            return cookies;
        }
        "#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load analyzed module data
    let profile = test.default_profile_id(module_id);
    let dir = test.artifact_dir(module_id, profile);
    let tree = &dir.tree;
    let found_assign = tree
        .iter_nodes_of_type::<Expression>()
        .find(|(_, expression)| matches!(expression, Expression::Assign { .. }));
    let Some((_, assign_expression)) = found_assign else {
        panic!("expected assignment expression");
    };

    let Expression::Assign { left, right } = assign_expression else {
        panic!("expected assignment expression");
    };
    let Expression::Index {
        left: indexed_value,
        right: Some(index_expression),
    } = tree.get(*left)
    else {
        panic!("expected computed index assignment target");
    };
    let Expression::Call {
        left: callee,
        arguments,
        ..
    } = tree.get(*index_expression)
    else {
        panic!("expected computed key call expression");
    };
    let Expression::Member {
        left: member_left,
        name,
        ..
    } = tree.get(*callee)
    else {
        panic!("expected trim member access");
    };
    let Expression::LocalReference {
        path: indexed_name, ..
    } = tree.get(*indexed_value)
    else {
        panic!("expected cookies local reference");
    };
    let Expression::LocalReference { path: key_name, .. } = tree.get(*member_left) else {
        panic!("expected key local reference");
    };
    let Expression::LocalReference {
        path: value_name, ..
    } = tree.get(*right)
    else {
        panic!("expected value local reference");
    };

    // assignment target shape
    let indexed_name = indexed_name
        .last_segment()
        .expect("expected cookies path segment");
    let key_name = key_name.last_segment().expect("expected key path segment");
    let value_name = value_name
        .last_segment()
        .expect("expected value path segment");

    assert_eq!(test.program.strings.get(indexed_name).as_ref(), "cookies");
    assert_eq!(test.program.strings.get(key_name).as_ref(), "key");
    assert_eq!(test.program.strings.get(value_name).as_ref(), "value");
    assert_eq!(arguments.len(), 0);

    let Some(member_name) = *name else {
        panic!("expected trim member name");
    };
    assert_eq!(test.program.strings.get(member_name).as_ref(), "trim");
}
