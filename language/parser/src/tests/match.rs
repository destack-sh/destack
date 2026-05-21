use destack_dir::{
    BinaryOperator, Block, CommentKind, Declarator, Expression, LetKind, MatchCase, MatchForm,
    MatchSelector, Mutability, NodeType, Pattern, PatternField, ScalarLiteral,
};
use destack_source::{LanguageType, NodeSpanRegion, NodeSpanType};

use crate::{
    TestParser, assert_comment, assert_expression_path, assert_node, assert_string,
    block_expression_ids,
};

#[test]
fn test_parse_match_simple_arms() {
    let mut test = TestParser::new(
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

    let match_id = parser.eat_match().unwrap();

    assert_node!(parser.tree, match_id, Expression::Match { form: MatchForm::Match, value, cases } => {
        // value: path x
        assert_expression_path!(parser, parser.tree.get(*value), "x");

        assert_eq!(cases.len(), 4);

        // case 0: 1 => 10
        assert_node!(parser.tree, cases[0], MatchCase::Expression { selector: MatchSelector::Pattern { pattern, guard }, body } => {
            assert!(guard.is_none());
            assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });
            assert_node!(parser.tree, *body, Expression::ScalarLiteral(ScalarLiteral::Integer(10)));
        });

        // case 1: 2 => 20
        assert_node!(parser.tree, cases[1], MatchCase::Expression { selector: MatchSelector::Pattern { pattern, guard }, body } => {
            assert!(guard.is_none());
            assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
            });
            assert_node!(parser.tree, *body, Expression::ScalarLiteral(ScalarLiteral::Integer(20)));
        });

        // case 2: x => x
        assert_node!(parser.tree, cases[2], MatchCase::Expression { selector: MatchSelector::Pattern { pattern, guard }, body: _ } => {
            assert!(guard.is_none());
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: _ } => {
                assert_string!(parser, *name, "x");
            });
        });

        // case 3: _ => 0
        assert_node!(parser.tree, cases[3], MatchCase::Expression { selector: MatchSelector::Pattern { pattern, guard }, body } => {
            assert!(guard.is_none());
            assert_node!(parser.tree, *pattern, Pattern::Wildcard);
            assert_node!(parser.tree, *body, Expression::ScalarLiteral(ScalarLiteral::Integer(0)));
        });
    });
}

#[test]
fn test_parse_match_object_pattern_arms_with_expression_bodies() {
    let mut test = TestParser::new(
        r#"
match (shape) {
    { kind: "circle", radius } => radius
    { kind: "square", size } => size
}
"#,
    );
    let mut parser = test.prepare();
    let match_id = parser.eat_match().unwrap();

    test.assert_no_errors(&parser);

    assert_node!(parser.tree, match_id, Expression::Match { cases, .. } => {
        assert_eq!(cases.len(), 2);

        // { kind: "circle", radius } => radius
        assert_node!(parser.tree, cases[0], MatchCase::Expression { selector: MatchSelector::Pattern { pattern, guard }, body } => {
            assert!(guard.is_none());
            assert_node!(parser.tree, *pattern, Pattern::Object { fields } => {
                assert_eq!(fields.len(), 2);
            });
            assert_expression_path!(parser, parser.tree.get(*body), "radius");
        });

        // { kind: "square", size } => size
        assert_node!(parser.tree, cases[1], MatchCase::Expression { selector: MatchSelector::Pattern { pattern, guard }, body } => {
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
    let mut test = TestParser::new(
        r###"
match (x) {
    2 if (true) => 20
}
"###,
    );
    let mut parser = test.prepare();

    let match_id = parser.eat_match().unwrap();
    assert_node!(parser.tree, match_id, Expression::Match { form: MatchForm::Match, value: _, cases } => {
        assert_eq!(cases.len(), 1);

        // case: 2 if (true) => 20
        assert_node!(parser.tree, cases[0], MatchCase::Expression { selector: MatchSelector::Pattern { pattern, guard }, body } => {
            let guard_clause_span = parser
                .tree
                .get_side_span(cases[0], NodeSpanType::Region(NodeSpanRegion::Clause))
                .expect("expected guard clause span");
            assert_eq!(parser.get_span_str(guard_clause_span), "if (true)");

            // guard: true
            let guard_id = guard.expect("expected guard");
            assert_node!(parser.tree, guard_id, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));

            // pattern: 2
            assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
            });

            // body: 20
            assert_node!(parser.tree, *body, Expression::ScalarLiteral(ScalarLiteral::Integer(20)));
        });
    });
}

#[test]
fn test_parse_match_tuple_guard_with_comparison() {
    let mut test = TestParser::new(
        r###"
match (pair) {
    (_, count) if (count > 0) => count
    _ => 0
}
"###,
    );
    let mut parser = test.prepare();

    let match_id = parser.eat_match().unwrap();
    assert_node!(parser.tree, match_id, Expression::Match { form: MatchForm::Match, value: _, cases } => {
        assert_eq!(cases.len(), 2);

        assert_node!(parser.tree, cases[0], MatchCase::Expression { selector: MatchSelector::Pattern { pattern, guard }, body } => {
            assert_node!(parser.tree, *pattern, Pattern::Tuple { fields } => {
                assert_eq!(fields.len(), 2);

                assert_node!(parser.tree, fields[0], PatternField::Positional { pattern } => {
                    assert_node!(parser.tree, *pattern, Pattern::Wildcard);
                });
                assert_node!(parser.tree, fields[1], PatternField::Named { name: _, is_shorthand, pattern } => {
                    assert!(*is_shorthand);
                    assert!(pattern.is_none());
                });
            });

            let guard_id = guard.expect("expected guard");
            assert_node!(parser.tree, guard_id, Expression::Binary { left, operator: BinaryOperator::GreaterThan, right } => {
                assert_expression_path!(parser, parser.tree.get(*left), "count");
                assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(0)));
            });
            assert_expression_path!(parser, parser.tree.get(*body), "count");
        });

        assert_node!(parser.tree, cases[1], MatchCase::Expression { selector: MatchSelector::Pattern { pattern, guard }, body } => {
            assert_node!(parser.tree, *pattern, Pattern::Wildcard);
            assert!(guard.is_none());
            assert_node!(parser.tree, *body, Expression::ScalarLiteral(ScalarLiteral::Integer(0)));
        });
    });
}

#[test]
fn test_parse_match_with_paths() {
    let mut test = TestParser::new(
        r"
match (self) {
    TetrisPieceShape.I => Color.Blue
    TetrisPieceShape.J => Color.Red
    _ => Color.Gray
}
    ",
    );
    let mut parser = test.prepare();

    let match_id = parser.eat_match().unwrap();

    // match (self) { ... }
    assert_node!(parser.tree, match_id, Expression::Match { form: MatchForm::Match, value, cases } => {
        // self
        assert_expression_path!(parser, parser.tree.get(*value), "self");

        assert_eq!(cases.len(), 3);

        // TetrisPieceShape.I => Color.Blue
        assert_node!(parser.tree, cases[0], MatchCase::Expression { selector: MatchSelector::Pattern { pattern, guard }, body } => {
            assert!(guard.is_none());
            // TetrisPieceShape.I
            assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                assert_expression_path!(parser, parser.tree.get(*value), "TetrisPieceShape.I");
            });
            // Color.Blue
            assert_expression_path!(parser, parser.tree.get(*body), "Color.Blue");
        });

        // TetrisPieceShape.J => Color.Red
        assert_node!(parser.tree, cases[1], MatchCase::Expression { selector: MatchSelector::Pattern { pattern, guard }, body } => {
            assert!(guard.is_none());
            // TetrisPieceShape.J
            assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                assert_expression_path!(parser, parser.tree.get(*value), "TetrisPieceShape.J");
            });
            // Color.Red
            assert_expression_path!(parser, parser.tree.get(*body), "Color.Red");
        });

        // _ => Color.Gray
        assert_node!(parser.tree, cases[2], MatchCase::Expression { selector: MatchSelector::Pattern { pattern, guard }, body } => {
            assert!(guard.is_none());
            assert_node!(parser.tree, *pattern, Pattern::Wildcard);
            // Color.Gray
            assert_expression_path!(parser, parser.tree.get(*body), "Color.Gray");
        });
    });
}

#[test]
fn test_parse_match_parenthesized_value_keeps_inner_span() {
    let mut test = TestParser::new(
        r"
match (value) {
    _ => result
}
",
    );
    let mut parser = test.prepare();

    let match_id = parser.eat_match().unwrap();
    assert_node!(parser.tree, match_id, Expression::Match { value, .. } => {
        assert_expression_path!(parser, parser.tree.get(*value), "value");

        let value_span = parser.tree.get_span(*value);
        let value_text = &parser.file.text()[value_span.start as usize..value_span.end as usize];
        assert_eq!(value_text, "value");
    });
}

#[test]
fn test_parse_match_arm_keeps_if_else_branch_semicolons_as_statements() {
    let mut test = TestParser::new(
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

    let match_id = parser.eat_match().unwrap();

    assert_node!(parser.tree, match_id, Expression::Match { cases, .. } => {
        assert_eq!(cases.len(), 2);

        assert_node!(parser.tree, cases[0], MatchCase::Block { body, .. } => {
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

/// Parse a switch-case statement.
#[test]
fn test_parse_match_from_switch_case() {
    let mut test = TestParser::new(
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

    let switch_id = parser.eat_match().unwrap();
    assert_node!(parser.tree, switch_id, Expression::Match { form: MatchForm::Switch, value: _, cases } => {
        assert_eq!(cases.len(), 4);

        // case "static" block
        assert_node!(parser.tree, cases[0], MatchCase::Block { selector: MatchSelector::Pattern { pattern, guard }, body } => {
            assert!(guard.is_none());
            // selector
            assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(literal)) => {
                    assert_string!(parser, *literal, "static");
                });
            });
            // body
            assert_node!(parser.tree, *body, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*body));
                assert_eq!(expressions.len(), 2);
            });
        });

        // case "dynamic" block
        assert_node!(parser.tree, cases[1], MatchCase::Block { selector: MatchSelector::Pattern { pattern, guard }, body } => {
            assert!(guard.is_none());
            // selector
            assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(literal)) => {
                    assert_string!(parser, *literal, "dynamic");
                });
            });
            // body
            assert_node!(parser.tree, *body, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*body));
                assert_eq!(expressions.len(), 2);
            });
        });

        // case "literal" expression
        assert_node!(parser.tree, cases[2], MatchCase::Expression { selector: MatchSelector::Pattern { pattern, guard }, body } => {
            assert!(guard.is_none());
            // selector
            assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(literal)) => {
                    assert_string!(parser, *literal, "literal");
                });
            });
            // body
            assert_node!(parser.tree, *body, Expression::Call { left, .. } => {
                assert_expression_path!(parser, parser.tree.get(*left), "something");
            });
        });

        // case default (block)
        assert_node!(parser.tree, cases[3], MatchCase::Block { selector: MatchSelector::Default, body } => {
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
    let mut test = TestParser::new(
        r###"
switch(a) { case 1: {}
/foo/ }
"###,
    );
    let mut parser = test.prepare();

    // switch(a) { case 1: {} /foo/ }
    let switch_id = parser.eat_match().unwrap();
    assert_node!(parser.tree, switch_id, Expression::Match { form: MatchForm::Switch, cases, .. } => {
        assert_eq!(cases.len(), 1);
        assert_node!(parser.tree, cases[0], MatchCase::Block { body, .. } => {
            assert_node!(parser.tree, *body, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*body));
                assert_eq!(expressions.len(), 2);
                assert_node!(parser.tree, expressions[0], Expression::Block(_));
                assert_node!(parser.tree, expressions[1], Expression::ScalarLiteral(ScalarLiteral::RegexString { .. }));
            });
        });
    });
}

/// Parse switch case statements that begin with a semicolon before a parenthesized call.
#[test]
fn test_parse_switch_case_leading_semicolon_parenthesized_call() {
    let mut test = TestParser::new(
        r###"
switch (tag.injectTo) {
  case "body":
    ;(bodyTags ??= []).push(tag)
    break
}
"###,
    );
    let mut parser = test.prepare();

    let switch_id = parser.eat_match().unwrap();
    assert_node!(parser.tree, switch_id, Expression::Match { form: MatchForm::Switch, cases, .. } => {
        assert_eq!(cases.len(), 1);
        assert_node!(parser.tree, cases[0], MatchCase::Block { body, .. } => {
            assert_node!(parser.tree, *body, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*body));
                assert_eq!(expressions.len(), 2);

                // first statement: (bodyTags ??= []).push(tag)
                assert_node!(parser.tree, expressions[0], Expression::Call { left, arguments, .. } => {
                    assert_eq!(arguments.len(), 1);
                            assert_node!(parser.tree, *left, Expression::Member { left, name, .. } => {
                                assert_string!(parser, *name, "push");
                                assert_node!(parser.tree, *left, Expression::Parenthesized { expression } => {
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
    let mut test = TestParser::new_with_language(
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
        LanguageType::JavaScript,
    );
    let mut parser = test.prepare();

    let switch_id = parser.eat_match().unwrap();
    assert_node!(parser.tree, switch_id, Expression::Match { form: MatchForm::Switch, cases, .. } => {
        assert_eq!(cases.len(), 2);

        // first case body
        assert_node!(parser.tree, cases[0], MatchCase::Block { selector, body } => {
            match selector {
                MatchSelector::Pattern { pattern, guard } => {
                    assert!(guard.is_none());
                    assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                        assert_expression_path!(parser, parser.tree.get(*value), "dataViewTag");
                    });
                }
                _ => panic!("expected pattern selector"),
            };
            assert_node!(parser.tree, *body, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*body));
                assert_eq!(expressions.len(), 3);
                assert_node!(parser.tree, expressions[0], Expression::If { .. });
                let first_assign_id = parser.unwrap_label_expression(expressions[1]);
                let second_assign_id = parser.unwrap_label_expression(expressions[2]);
                assert_node!(parser.tree, first_assign_id, Expression::Assign { .. });
                assert_node!(parser.tree, second_assign_id, Expression::Assign { .. });
            });
        });

        // second case body
        assert_node!(parser.tree, cases[1], MatchCase::Expression { selector, body } => {
            match selector {
                MatchSelector::Pattern { pattern, guard } => {
                    assert!(guard.is_none());
                    assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                        assert_expression_path!(parser, parser.tree.get(*value), "arrayBufferTag");
                    });
                }
                _ => panic!("expected pattern selector"),
            };
            let return_id = parser.unwrap_label_expression(*body);
            assert_node!(parser.tree, return_id, Expression::Return { .. });
        });
    });
}

/// Parse switch case statements that continue after one break statement.
#[test]
fn test_parse_switch_case_with_multiple_break_statements() {
    let mut test = TestParser::new(
        r###"
switch (value) {
  case 1:
    break;
    break;
}
"###,
    );
    let mut parser = test.prepare();

    let switch_id = parser.eat_match().unwrap();
    assert_node!(parser.tree, switch_id, Expression::Match { form: MatchForm::Switch, cases, .. } => {
        assert_eq!(cases.len(), 1);
        assert_node!(parser.tree, cases[0], MatchCase::Block { body, .. } => {
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
    let mut test = TestParser::new_with_language(
        "switch (cond) { case 10: let a = 20;",
        LanguageType::JavaScript,
    );
    let mut parser = test.prepare();
    let roots = parser.parse();

    assert_eq!(roots.len(), 1);
    assert_node!(parser.tree, roots[0], Expression::Match { form, value, cases } => {
        assert_eq!(*form, MatchForm::Switch);
        assert_expression_path!(parser, parser.tree.get(*value), "cond");
        assert_eq!(cases.len(), 1);
        assert_node!(parser.tree, cases[0], MatchCase::Expression { selector, body } => {
            assert_node!(selector, MatchSelector::Pattern { pattern, guard } => {
                assert!(guard.is_none());
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(10)));
                });
            });
            assert_node!(parser.tree, *body, Expression::Let { kind, export, mutability, declarators, is_ambient, is_shared } => {
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
                    assert_node!(parser.tree, value.expect("expected initializer"), Expression::ScalarLiteral(ScalarLiteral::Integer(20)));
                });
            });
        });
    });

    test.assert_error_leaves(&parser, &[(Some(NodeType::MatchCase), None, "")]);
}

/// Parse minified switch cases where `continue` is followed by `}` and another `if`.
#[test]
fn test_parse_switch_case_minified_if_continue_then_if() {
    let mut test = TestParser::new_with_language(
        "switch(op[0]){default:if(!(t=_.trys,t=t.length>0&&t[t.length-1])&&(op[0]===6||op[0]===2)){_=0;continue}if(op[0]===3&&(!t||op[1]>t[0]&&op[1]<t[3])){_.label=op[1];break}}",
        LanguageType::JavaScript,
    );
    let mut parser = test.prepare();
    let switch_id = parser.eat_match().unwrap();

    // switch(op[0]) { default: if (...) { _ = 0; continue } if (...) { _.label = op[1]; break } }
    assert_node!(parser.tree, switch_id, Expression::Match { form: MatchForm::Switch, cases, .. } => {
        assert_eq!(cases.len(), 1);

        // default case body keeps both if statements
        assert_node!(parser.tree, cases[0], MatchCase::Block { selector, body } => {
            assert!(matches!(selector, MatchSelector::Default));
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
    let mut test = TestParser::new_with_language(
        r#"switch (state) {
  // before-ready
  case "ready":
    start() // ready-tail
    break
  default:
    stop() // default-tail
}"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert!(
        parser.errors.is_empty(),
        "unexpected parser errors: {:?}",
        parser.errors
    );
    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_label_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Match { form, cases, .. } => {
        assert_eq!(*form, MatchForm::Switch);
        assert_eq!(cases.len(), 2);

        let first_case_annotations = parser.tree.get_decorators(cases[0].id);
        assert!(first_case_annotations.is_empty());

        assert_node!(parser.tree, cases[0], MatchCase::Block { body, .. } => {
            assert_node!(parser.tree, *body, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*body));
                assert_eq!(expressions.len(), 2);
                let start_annotations = parser.tree.get_decorators(expressions[0].id);
                assert!(start_annotations.is_empty());
            });
        });

        assert_node!(parser.tree, cases[1], MatchCase::Expression { body, .. } => {
            let default_annotations = parser.tree.get_decorators(body.id);
            assert!(default_annotations.is_empty());
        });
    });
    assert_eq!(parser.tree.comments().len(), 3);
    assert_comment!(parser, 0, CommentKind::Line, "before-ready");

    assert_comment!(parser, 1, CommentKind::Line, "ready-tail");

    assert_comment!(parser, 2, CommentKind::Line, "default-tail");
}
