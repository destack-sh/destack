use tspp_dir::{
    BinaryOperator, Block, CommentKind, ConditionOperand, Declarator, Expression, LetKind, Literal,
    MatchArm, Mutability, NodeType, Pattern, PatternField, SwitchCase, SwitchSelector, TokenType,
};
use tspp_source::{NodeSpanRegion, NodeSpanType};

use crate::{
    TestParser, assert_comment, assert_expression_path, assert_node, assert_string,
    block_expression_ids,
};

/// Keep repeated match-arm placeholders outside ordinary TS++ grammar.
#[test]
fn test_reject_match_arm_placeholder_in_tspp_grammar() {
    let test = TestParser::new("match (value) { $$$ARMS }");
    let mut parser = test.prepare();
    parser.parse_in_place();

    test.assert_errors(
        &parser,
        &[
            (
                Some(NodeType::Expression),
                Some(TokenType::CloseBrace),
                Some(TokenType::ArrowWide),
                "}",
            ),
            (None, Some(TokenType::CloseBrace), None, "}"),
        ],
    );
}

/// Parse repeated Pattern placeholders as complete match arms and switch cases.
#[test]
fn test_parse_pattern_branch_placeholders() {
    let test = TestParser::new("match (value) { $$$ARMS }");
    let mut parser = test.prepare_pattern();
    let roots = parser.parse_in_place();

    test.assert_no_errors(&parser);
    assert_node!(parser.tree, roots[0], Expression::Match { arms, .. } => {
        assert_eq!(arms.len(), 1);
        assert_node!(parser.tree, arms[0], MatchArm::Expression { pattern, body, .. } => {
            assert_node!(parser.tree, *pattern, Pattern::Wildcard);
            assert_node!(parser.tree, *body, Expression::Error);
        });
    });

    let test = TestParser::new("switch (value) { $$$CASES }");
    let mut parser = test.prepare_pattern();
    let roots = parser.parse_in_place();

    test.assert_no_errors(&parser);
    assert_node!(parser.tree, roots[0], Expression::Switch { cases, .. } => {
        assert_eq!(cases.len(), 1);
        assert_node!(parser.tree, cases[0], SwitchCase {
            selector: SwitchSelector::Default,
            body,
        } => {
            assert_node!(parser.tree, *body, Block {
                leading_expressions,
                tail_expression: None,
                ..
            } => {
                assert!(leading_expressions.is_empty());
            });
        });
    });
}

#[test]
fn test_parse_match_simple_arms() {
    let test = TestParser::new(
        r###"
match (x) {
    1 => 10
    2 => 20
    x => x
    _ => 0
}
"###,
    );
    let mut parser = test.prepare();

    let match_id = parser.parse_match().unwrap();
    let main_span = parser
        .tree
        .get_main_span(match_id)
        .expect("expected match keyword main span");
    assert_eq!(parser.span_str(main_span), "match");

    assert_node!(parser.tree, match_id, Expression::Match { value, arms } => {
        // value: path x
        assert_expression_path!(parser, parser.tree.get(*value), "x");

        assert_eq!(arms.len(), 4);

        // case 0: 1 => 10
        assert_node!(parser.tree, arms[0], MatchArm::Expression { pattern, guard, body } => {
            assert!(guard.is_none());
            assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(1)));
            });
            assert_node!(parser.tree, *body, Expression::Literal(Literal::Integer(10)));
        });

        // case 1: 2 => 20
        assert_node!(parser.tree, arms[1], MatchArm::Expression { pattern, guard, body } => {
            assert!(guard.is_none());
            assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(2)));
            });
            assert_node!(parser.tree, *body, Expression::Literal(Literal::Integer(20)));
        });

        // case 2: x => x
        assert_node!(parser.tree, arms[2], MatchArm::Expression { pattern, guard, body: _ } => {
            assert!(guard.is_none());
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: _ } => {
                assert_string!(parser, *name, "x");
            });
        });

        // case 3: _ => 0
        assert_node!(parser.tree, arms[3], MatchArm::Expression { pattern, guard, body } => {
            assert!(guard.is_none());
            assert_node!(parser.tree, *pattern, Pattern::Wildcard);
            assert_node!(parser.tree, *body, Expression::Literal(Literal::Integer(0)));
        });
    });
}

#[test]
fn test_parse_match_object_pattern_arms_with_expression_bodies() {
    let test = TestParser::new(
        r#"
match (shape) {
    { kind: "circle", radius } => radius
    { kind: "square", size } => size
}
"#,
    );
    let mut parser = test.prepare();
    let match_id = parser.parse_match().unwrap();

    test.assert_no_errors(&parser);

    assert_node!(parser.tree, match_id, Expression::Match { arms, .. } => {
        assert_eq!(arms.len(), 2);

        // { kind: "circle", radius } => radius
        assert_node!(parser.tree, arms[0], MatchArm::Expression { pattern, guard, body } => {
            assert!(guard.is_none());
            assert_node!(parser.tree, *pattern, Pattern::Object { fields } => {
                assert_eq!(fields.len(), 2);
            });
            assert_expression_path!(parser, parser.tree.get(*body), "radius");
        });

        // { kind: "square", size } => size
        assert_node!(parser.tree, arms[1], MatchArm::Expression { pattern, guard, body } => {
            assert!(guard.is_none());
            assert_node!(parser.tree, *pattern, Pattern::Object { fields } => {
                assert_eq!(fields.len(), 2);
            });
            assert_expression_path!(parser, parser.tree.get(*body), "size");
        });
    });
}

#[test]
fn test_parse_match_with_guard() {
    let test = TestParser::new(
        r###"
match (x) {
    2 if (true) => 20
}
"###,
    );
    let mut parser = test.prepare();

    let match_id = parser.parse_match().unwrap();
    assert_node!(parser.tree, match_id, Expression::Match { value: _, arms } => {
        assert_eq!(arms.len(), 1);

        // case: 2 if (true) => 20
        assert_node!(parser.tree, arms[0], MatchArm::Expression { pattern, guard, body } => {
            let guard_clause_span = parser
                .tree
                .get_side_span(arms[0], NodeSpanType::Region(NodeSpanRegion::Guard))
                .expect("expected guard clause span");
            assert_eq!(parser.span_str(guard_clause_span), "if (true)");

            // guard: true
            let guard_id = guard
                .as_ref()
                .expect("expected guard")
                .as_expression()
                .expect("expected expression guard");
            assert_node!(parser.tree, guard_id, Expression::Literal(Literal::Boolean(true)));

            // pattern: 2
            assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(2)));
            });

            // body: 20
            assert_node!(parser.tree, *body, Expression::Literal(Literal::Integer(20)));
        });
    });
}

#[test]
fn test_parse_match_tuple_guard_with_comparison() {
    let test = TestParser::new(
        r###"
match (pair) {
    (_, count) if (count > 0) => count
    _ => 0
}
"###,
    );
    let mut parser = test.prepare();

    let match_id = parser.parse_match().unwrap();
    assert_node!(parser.tree, match_id, Expression::Match { value: _, arms } => {
        assert_eq!(arms.len(), 2);

        assert_node!(parser.tree, arms[0], MatchArm::Expression { pattern, guard, body } => {
            assert_node!(parser.tree, *pattern, Pattern::Tuple { fields } => {
                assert_eq!(fields.len(), 2);

                assert_node!(parser.tree, fields[0], PatternField::Positional { pattern } => {
                    assert_node!(parser.tree, *pattern, Pattern::Wildcard);
                });
                assert_node!(parser.tree, fields[1], PatternField::Positional { pattern } => {
                    assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None } => {
                        assert_string!(parser, *name, "count");
                    });
                });
            });

            let guard_id = guard
                .as_ref()
                .expect("expected guard")
                .as_expression()
                .expect("expected expression guard");
            assert_node!(parser.tree, guard_id, Expression::Binary { left, operator: BinaryOperator::GreaterThan, right } => {
                assert_expression_path!(parser, parser.tree.get(*left), "count");
                assert_node!(parser.tree, *right, Expression::Literal(Literal::Integer(0)));
            });
            assert_expression_path!(parser, parser.tree.get(*body), "count");
        });

        assert_node!(parser.tree, arms[1], MatchArm::Expression { pattern, guard, body } => {
            assert_node!(parser.tree, *pattern, Pattern::Wildcard);
            assert!(guard.is_none());
            assert_node!(parser.tree, *body, Expression::Literal(Literal::Integer(0)));
        });
    });
}

/// Parse binding and expression operands in one match guard.
#[test]
fn test_parse_match_binding_guard() {
    let test = TestParser::new(
        r#"
match (input) {
    text if (let value! = parse(text) && value > 0) => value
}
"#,
    );
    let mut parser = test.prepare();

    let match_id = parser.parse_match().unwrap();
    assert_node!(parser.tree, match_id, Expression::Match { arms, .. } => {
        assert_eq!(arms.len(), 1);
        assert_node!(parser.tree, arms[0], MatchArm::Expression { guard: Some(guard), body, .. } => {
            let guard_span = parser
                .tree
                .get_side_span(arms[0], NodeSpanType::Region(NodeSpanRegion::Guard))
                .expect("expected guard clause span");
            assert_eq!(
                parser.span_str(guard_span),
                "if (let value! = parse(text) && value > 0)"
            );

            assert_eq!(guard.operands.len(), 2);
            let ConditionOperand::Binding { kind, declarator, .. } = &guard.operands[0] else {
                panic!("expected binding operand");
            };
            assert_eq!(*kind, LetKind::Let);
            assert_node!(parser.tree, *declarator, Declarator { pattern, value: Some(value), .. } => {
                assert_node!(parser.tree, *pattern, Pattern::Must(pattern) => {
                    assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None } => {
                        assert_string!(parser, *name, "value");
                    });
                });
                assert_node!(parser.tree, *value, Expression::Call { left, arguments, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "parse");
                    assert_eq!(arguments.len(), 1);
                    let argument = parser
                        .tree
                        .get(arguments[0])
                        .value()
                        .expect("expected argument value");
                    assert_expression_path!(parser, parser.tree.get(argument), "text");
                });
            });
            let ConditionOperand::Expression { condition } = &guard.operands[1] else {
                panic!("expected expression operand");
            };
            assert_node!(parser.tree, *condition, Expression::Binary { left, operator: BinaryOperator::GreaterThan, right } => {
                assert_expression_path!(parser, parser.tree.get(*left), "value");
                assert_node!(parser.tree, *right, Expression::Literal(Literal::Integer(0)));
            });
            assert_expression_path!(parser, parser.tree.get(*body), "value");
        });
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_match_with_paths() {
    let test = TestParser::new(
        r"
match (self) {
    TetrisPieceShape.I => Color.Blue
    TetrisPieceShape.J => Color.Red
    _ => Color.Gray
}
    ",
    );
    let mut parser = test.prepare();

    let match_id = parser.parse_match().unwrap();

    // match (self) { ... }
    assert_node!(parser.tree, match_id, Expression::Match { value, arms } => {
        // self
        assert_expression_path!(parser, parser.tree.get(*value), "self");

        assert_eq!(arms.len(), 3);

        // TetrisPieceShape.I => Color.Blue
        assert_node!(parser.tree, arms[0], MatchArm::Expression { pattern, guard, body } => {
            assert!(guard.is_none());
            // TetrisPieceShape.I
            assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                assert_expression_path!(parser, parser.tree.get(*value), "TetrisPieceShape.I");
            });
            // Color.Blue
            assert_expression_path!(parser, parser.tree.get(*body), "Color.Blue");
        });

        // TetrisPieceShape.J => Color.Red
        assert_node!(parser.tree, arms[1], MatchArm::Expression { pattern, guard, body } => {
            assert!(guard.is_none());
            // TetrisPieceShape.J
            assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                assert_expression_path!(parser, parser.tree.get(*value), "TetrisPieceShape.J");
            });
            // Color.Red
            assert_expression_path!(parser, parser.tree.get(*body), "Color.Red");
        });

        // _ => Color.Gray
        assert_node!(parser.tree, arms[2], MatchArm::Expression { pattern, guard, body } => {
            assert!(guard.is_none());
            assert_node!(parser.tree, *pattern, Pattern::Wildcard);
            // Color.Gray
            assert_expression_path!(parser, parser.tree.get(*body), "Color.Gray");
        });
    });
}

#[test]
fn test_parse_match_keeps_expression_and_value_spans() {
    let test = TestParser::new(
        r"
match (value) {
    _ => result
}
",
    );
    let mut parser = test.prepare();

    let match_id = parser.parse_match().unwrap();
    assert_node!(parser.tree, match_id, Expression::Match { value, .. } => {
        let match_span = parser.tree.get_span(match_id);
        assert_eq!(parser.span_str(match_span), "match (value) {\n    _ => result\n}");

        assert_expression_path!(parser, parser.tree.get(*value), "value");

        let value_span = parser.tree.get_span(*value);
        let value_text = &parser.file.text()[value_span.start as usize..value_span.end as usize];
        assert_eq!(value_text, "value");
    });
}

#[test]
fn test_parse_switch_records_expression_and_selector_spans() {
    let test = TestParser::new("switch (value) { case 1: break; default: break }");
    let mut parser = test.prepare();

    let switch_id = parser.parse_switch().unwrap();
    let switch_span = parser.tree.get_span(switch_id);
    let main_span = parser
        .tree
        .get_main_span(switch_id)
        .expect("expected switch keyword main span");

    assert_eq!(
        parser.span_str(switch_span),
        "switch (value) { case 1: break; default: break }"
    );
    assert_eq!(parser.span_str(main_span), "switch");

    assert_node!(parser.tree, switch_id, Expression::Switch { cases, .. } => {
        let case_span = parser
            .tree
            .get_main_span(cases[0])
            .expect("expected switch selector main span");
        assert_eq!(parser.span_str(case_span), "case 1");

        let default_span = parser
            .tree
            .get_main_span(cases[1])
            .expect("expected switch selector main span");
        assert_eq!(parser.span_str(default_span), "default");
    });
}

#[test]
fn test_parse_match_arm_keeps_if_else_branch_semicolons_as_statements() {
    let test = TestParser::new(
        r#"
match (result) {
    Ok(value) => {
        if (value.valid) { use(value); } else { reset(); }
    };
    Err(error) => report(error)
}
"#,
    );
    let mut parser = test.prepare();

    let match_id = parser.parse_match().unwrap();

    assert_node!(parser.tree, match_id, Expression::Match { arms, .. } => {
        assert_eq!(arms.len(), 2);

        assert_node!(parser.tree, arms[0], MatchArm::Block { body, .. } => {
            let body_block = parser.tree.get(*body);
            assert!(body_block.leading_expressions.is_empty());
            let if_expression_id = body_block
                .tail_expression
                .expect("expected match arm tail expression");

            assert_node!(parser.tree, if_expression_id, Expression::If { then_expression, else_expression, .. } => {
                assert_node!(parser.tree, *then_expression, Expression::Block(then_block_id) => {
                    let then_block = parser.tree.get(*then_block_id);
                    assert_eq!(then_block.leading_expressions.len(), 1);
                    assert!(then_block.tail_expression.is_none());
                });

                let else_expression = else_expression.expect("expected else expression");
                assert_node!(parser.tree, else_expression, Expression::Block(else_block_id) => {
                    let else_block = parser.tree.get(*else_block_id);
                    assert_eq!(else_block.leading_expressions.len(), 1);
                    assert!(else_block.tail_expression.is_none());
                });
            });
        });
    });
}

#[test]
fn test_parse_switch_cases() {
    let test = TestParser::new(
        r###"
switch (left.type) {
    case "static":
        left.field;
        break;
    case "dynamic":
        left.value;
        something();
        // implicitly break
    case "literal":
        something();
        // implicitly break, one statement (no block
    default:
        something();
        return left.name;
  }
"###,
    );
    let mut parser = test.prepare();

    let switch_id = parser.parse_switch().unwrap();
    assert_node!(parser.tree, switch_id, Expression::Switch { value: _, cases } => {
        assert_eq!(cases.len(), 4);

        // case "static" block
        assert_node!(parser.tree, cases[0], SwitchCase { selector: SwitchSelector::Case(value), body } => {
            // selector
            assert_node!(parser.tree, *value, Expression::Literal(Literal::String(literal)) => {
                assert_string!(parser, *literal, "static");
            });
            // body
            assert_node!(parser.tree, *body, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*body));
                assert_eq!(expressions.len(), 2);
            });
        });

        // case "dynamic" block
        assert_node!(parser.tree, cases[1], SwitchCase { selector: SwitchSelector::Case(value), body } => {
            // selector
            assert_node!(parser.tree, *value, Expression::Literal(Literal::String(literal)) => {
                assert_string!(parser, *literal, "dynamic");
            });
            // body
            assert_node!(parser.tree, *body, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*body));
                assert_eq!(expressions.len(), 2);
            });
        });

        // case "literal" block
        assert_node!(parser.tree, cases[2], SwitchCase { selector: SwitchSelector::Case(value), body } => {
            // selector
            assert_node!(parser.tree, *value, Expression::Literal(Literal::String(literal)) => {
                assert_string!(parser, *literal, "literal");
            });
            // body
            let expressions = block_expression_ids(parser.tree.get(*body));
            assert_eq!(expressions.len(), 1);
            assert_node!(parser.tree, expressions[0], Expression::Call { left, .. } => {
                assert_expression_path!(parser, parser.tree.get(*left), "something");
            });
        });

        // case default (block)
        assert_node!(parser.tree, cases[3], SwitchCase { selector: SwitchSelector::Default, body } => {
            // body
            assert_node!(parser.tree, *body, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*body));
                assert_eq!(expressions.len(), 2);
            });
        });
    });
}

/// Parse a switch case with a block statement followed by a regex expression statement.
#[test]
fn test_parse_switch_case_block_then_regex_expression_statement() {
    let test = TestParser::new(
        r###"
switch(a) { case 1: {}
/foo/ }
"###,
    );
    let mut parser = test.prepare();

    // switch(a) { case 1: {} /foo/ }
    let switch_id = parser.parse_switch().unwrap();
    assert_node!(parser.tree, switch_id, Expression::Switch { cases, .. } => {
        assert_eq!(cases.len(), 1);
        assert_node!(parser.tree, cases[0], SwitchCase { body, .. } => {
            assert_node!(parser.tree, *body, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*body));
                assert_eq!(expressions.len(), 2);
                assert_node!(parser.tree, expressions[0], Expression::Block(_));
                assert_node!(parser.tree, expressions[1], Expression::Literal(Literal::RegexString { .. }));
            });
        });
    });
}

/// Parse switch case statements that begin with a semicolon before a parenthesized call.
#[test]
fn test_parse_switch_case_leading_semicolon_parenthesized_call() {
    let test = TestParser::new(
        r###"
switch (tag.injectTo) {
  case "body":
    ;(bodyTags ??= []).push(tag)
    break
}
"###,
    );
    let mut parser = test.prepare();

    let switch_id = parser.parse_switch().unwrap();
    assert_node!(parser.tree, switch_id, Expression::Switch { cases, .. } => {
        assert_eq!(cases.len(), 1);
        assert_node!(parser.tree, cases[0], SwitchCase { body, .. } => {
            assert_node!(parser.tree, *body, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*body));
                assert_eq!(expressions.len(), 2);

                // first statement: (bodyTags ??= []).push(tag)
                assert_node!(parser.tree, expressions[0], Expression::Call { left, arguments, .. } => {
                    assert_eq!(arguments.len(), 1);
                            assert_node!(parser.tree, *left, Expression::Member { left, name, .. } => {
                                assert_string!(parser, *name, "push");
                                crate::assert_parenthesized!(parser.tree, *left, expression => {
                                    assert_node!(parser.tree, *expression, Expression::Assign { left, right, .. } => {
                                assert_expression_path!(parser, parser.tree.get(*left), "bodyTags");
                                        assert_node!(parser.tree, *right, Expression::ArrayExpression { elements } => {
                                            assert!(elements.is_empty());
                                        });
                                    });
                                });
                    });
                });

                // second statement: break
                assert_node!(parser.tree, expressions[1], Expression::Break { .. });
            });
        });
    });
}

/// Parse a switch case with an if block, following assignments, then fallthrough case.
#[test]
fn test_parse_switch_case_if_block_then_assignments_then_fallthrough_case() {
    let test = TestParser::new(
        r###"
switch (tag) {
  case dataViewTag:
    if ((object.byteLength != other.byteLength) ||
        (object.byteOffset != other.byteOffset)) {
      return false;
    }
    object = object.buffer;
    other = other.buffer;

  case arrayBufferTag:
    return true;
}
"###,
    );
    let mut parser = test.prepare();

    let switch_id = parser.parse_switch().unwrap();
    assert_node!(parser.tree, switch_id, Expression::Switch { cases, .. } => {
        assert_eq!(cases.len(), 2);

        // first case body
        assert_node!(parser.tree, cases[0], SwitchCase { selector, body } => {
            match selector {
                SwitchSelector::Case(value) => {
                    assert_expression_path!(parser, parser.tree.get(*value), "dataViewTag");
                }
                SwitchSelector::Default => panic!("expected case selector"),
            };
            assert_node!(parser.tree, *body, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*body));
                assert_eq!(expressions.len(), 3);
                assert_node!(parser.tree, expressions[0], Expression::If { .. });
                let first_assign_id = expressions[1];
                let second_assign_id = expressions[2];
                assert_node!(parser.tree, first_assign_id, Expression::Assign { .. });
                assert_node!(parser.tree, second_assign_id, Expression::Assign { .. });
            });
        });

        // second case body
        assert_node!(parser.tree, cases[1], SwitchCase { selector, body } => {
            match selector {
                SwitchSelector::Case(value) => {
                    assert_expression_path!(parser, parser.tree.get(*value), "arrayBufferTag");
                }
                SwitchSelector::Default => panic!("expected case selector"),
            };
            let expressions = block_expression_ids(parser.tree.get(*body));
            assert_eq!(expressions.len(), 1);
            let return_id = expressions[0];
            assert_node!(parser.tree, return_id, Expression::Return { .. });
        });
    });
}

/// Parse switch case statements that continue after one break statement.
#[test]
fn test_parse_switch_case_with_multiple_break_statements() {
    let test = TestParser::new(
        r###"
switch (value) {
  case 1:
    break;
    break;
}
"###,
    );
    let mut parser = test.prepare();

    let switch_id = parser.parse_switch().unwrap();
    assert_node!(parser.tree, switch_id, Expression::Switch { cases, .. } => {
        assert_eq!(cases.len(), 1);
        assert_node!(parser.tree, cases[0], SwitchCase { body, .. } => {
            assert_node!(parser.tree, *body, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*body));
                assert_eq!(expressions.len(), 2);
                assert_node!(parser.tree, expressions[0], Expression::Break { .. });
                assert_node!(parser.tree, expressions[1], Expression::Break { .. });
            });
        });
    });
}

/// Recover a switch case body that reaches EOF before the switch closes.
#[test]
fn test_parse_switch_case_body_recovers_at_eof() {
    let test = TestParser::new("switch (cond) { case 10: let a = 20;");
    let mut parser = test.prepare();
    let roots = parser.parse_in_place();

    assert_eq!(roots.len(), 1);
    assert_node!(parser.tree, roots[0], Expression::Switch { value, cases } => {
        assert_expression_path!(parser, parser.tree.get(*value), "cond");
        assert_eq!(cases.len(), 1);
        assert_node!(parser.tree, cases[0], SwitchCase { selector: SwitchSelector::Case(value), body } => {
            assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(10)));
            let expressions = block_expression_ids(parser.tree.get(*body));
            assert_eq!(expressions.len(), 1);
            assert_node!(parser.tree, expressions[0], Expression::Let { kind, export, mutability, declarators, is_ambient, is_shared } => {
                assert_eq!(*kind, LetKind::Let);
                assert_eq!(*export, None);
                assert_eq!(*mutability, Mutability::Mutable);
                assert!(!*is_ambient);
                assert!(!*is_shared);
                assert_eq!(declarators.len(), 1);
                assert_node!(parser.tree, declarators[0], Declarator { pattern, ty, value } => {
                    assert!(ty.is_none());
                    assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern } => {
                        assert_string!(parser, *name, "a");
                        assert!(pattern.is_none());
                    });
                    assert_node!(parser.tree, value.expect("expected initializer"), Expression::Literal(Literal::Integer(20)));
                });
            });
        });
    });

    test.assert_errors(
        &parser,
        &[(
            Some(NodeType::SwitchCase),
            Some(TokenType::End),
            Some(TokenType::CloseBrace),
            "",
        )],
    );
}

/// Parse minified switch cases where `continue` is followed by `}` and another `if`.
#[test]
fn test_parse_switch_case_minified_if_continue_then_if() {
    let test = TestParser::new(
        "switch(op[0]){default:if(!(t=_.trys,t=t.length>0&&t[t.length-1])&&(op[0]===6||op[0]===2)){_=0;continue}if(op[0]===3&&(!t||op[1]>t[0]&&op[1]<t[3])){_.label=op[1];break}}",
    );
    let mut parser = test.prepare();
    let switch_id = parser.parse_switch().unwrap();

    // switch(op[0]) { default: if (...) { _ = 0; continue } if (...) { _.label = op[1]; break } }
    assert_node!(parser.tree, switch_id, Expression::Switch { cases, .. } => {
        assert_eq!(cases.len(), 1);

        // default case body keeps both if statements
        assert_node!(parser.tree, cases[0], SwitchCase { selector, body } => {
            assert!(matches!(selector, SwitchSelector::Default));
            assert_node!(parser.tree, *body, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*body));
                assert_eq!(expressions.len(), 2);
                assert_node!(parser.tree, expressions[0], Expression::If { .. });
                assert_node!(parser.tree, expressions[1], Expression::If { .. });
            });
        });
    });
}

#[test]
fn test_parse_switch_case_boundary_comment_ownership() {
    let test = TestParser::new(
        r#"switch (state) {
  // before-ready
  case "ready":
    start() // ready-tail
    break
  default:
    stop() // default-tail
}"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    assert!(
        parser.errors.is_empty(),
        "unexpected parser errors: {:?}",
        parser.errors
    );
    assert_eq!(expressions.len(), 1);

    let expression_id = expressions[0];
    assert_node!(parser.tree, expression_id, Expression::Switch { cases, .. } => {
        assert_eq!(cases.len(), 2);

        let first_case_annotations = parser.tree.get_decorators(cases[0].id);
        assert!(first_case_annotations.is_empty());

        assert_node!(parser.tree, cases[0], SwitchCase { body, .. } => {
            assert_node!(parser.tree, *body, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*body));
                assert_eq!(expressions.len(), 2);
                let start_annotations = parser.tree.get_decorators(expressions[0].id);
                assert!(start_annotations.is_empty());
            });
        });

        assert_node!(parser.tree, cases[1], SwitchCase { body, .. } => {
            let default_annotations = parser.tree.get_decorators(body.id);
            assert!(default_annotations.is_empty());
        });
    });
    assert_eq!(parser.comments().len(), 3);
    assert_comment!(parser, 0, CommentKind::Line, "before-ready");

    assert_comment!(parser, 1, CommentKind::Line, "ready-tail");

    assert_comment!(parser, 2, CommentKind::Line, "default-tail");
}
