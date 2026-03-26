use super::*;

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
            Expression::Statement { statement } => match view.tree().get(*statement) {
                Expression::If { .. } => Some(*statement),
                _ => None,
            },
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
    let module = test.program.modules.get(module_id);
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
            Expression::Statement { statement } => match tree.get(*statement) {
                Expression::If { .. } => Some(*statement),
                _ => None,
            },
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

    let Expression::Call {
        dynamic_arguments, ..
    } = tree.get(*right)
    else {
        panic!("expected call expression");
    };
    let argument_id = dynamic_arguments
        .first()
        .copied()
        .expect("expected call argument");
    let argument = tree.get(argument_id);
    let argument_value_id = argument.value();

    // ensure the right side reference uses a distinct node id
    assert_ne!(*left, argument_value_id);

    // build flow data for the condition expression
    // build the flow graph
    let graph = FlowGraphBuilder::new(module.id, &tree).build(condition_id);
    let context = InferState::new(profile, AnalyzeOptions::from(&CompilerOptions::default()));
    let mut flow_ctx = TypeContext::new(
        &module,
        profile,
        &context.options,
        &tree,
        &symbols,
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
            Expression::Declaration { declaration } => match view.tree().get(*declaration) {
                Declaration::Function {
                    body: Some(body), ..
                } => Some(*body),
                _ => None,
            },
            Expression::Statement { statement } => match view.tree().get(*statement) {
                Expression::Declaration { declaration } => match view.tree().get(*declaration) {
                    Declaration::Function {
                        body: Some(body), ..
                    } => Some(*body),
                    _ => None,
                },
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
    let view = test.view(module_id);
    let request_body_symbol = test.resolve_to_symbol("test.ts", "requestBody").unwrap();
    let redirect_allowed_symbol = test
        .resolve_to_symbol("test.ts", "redirectAllowed")
        .unwrap();

    // request body stays string after union narrowing
    let request_body_type = view.types().get_value_type(request_body_symbol).unwrap();
    assert_eq!(
        *request_body_type,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String)
        }
    );

    // redirect result stays boolean after loop convergence
    let redirect_allowed_type = view
        .types()
        .get_value_type(redirect_allowed_symbol)
        .unwrap();
    assert_eq!(
        *redirect_allowed_type,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Boolean)
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
        dynamic_arguments,
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
    assert_eq!(dynamic_arguments.len(), 0);

    let Some(member_name) = *name else {
        panic!("expected trim member name");
    };
    assert_eq!(test.program.strings.get(member_name).as_ref(), "trim");
}
