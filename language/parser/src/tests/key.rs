use destack_dir::{Expression, IfCondition, IfForm, Key, ScalarLiteral};
use destack_source::LanguageType;

use crate::tests::TestParser;
use crate::{assert_expression_path, assert_node, assert_string};

#[test]
fn test_reject_key_named_type_expression_with_multiline_type() {
    let mut test = TestParser::new(
        r#"[key:
    | string
    | number]"#,
    );
    let mut parser = test.prepare();
    let error = parser.eat_key_with_span().unwrap_err();
    assert_eq!(parser.get_span_str(error.leaf_span()), ":");
}

#[test]
fn test_reject_key_named_type_expression_with_newlines_before_colon_and_close_bracket() {
    let mut test = TestParser::new(
        r#"[key
:
string
]"#,
    );
    let mut parser = test.prepare();
    let error = parser.eat_key_with_span().unwrap_err();
    assert_eq!(parser.get_span_str(error.leaf_span()), ":");
}

/// Reject computed keys with sequence expressions in typed and untyped object forms.
#[test]
fn test_reject_key_computed_sequence_expression_in_typed_and_untyped_object_forms() {
    // source: [a,b]
    let mut test = TestParser::new_with_language("[a,b]", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let error = parser.eat_key_with_span().unwrap_err();

    // ,
    assert_eq!(parser.get_span_str(error.leaf_span()), ",");
}

/// Reject legacy octal numeric keys in typed and untyped object forms.
#[test]
fn test_reject_legacy_octal_numeric_key_in_typed_and_untyped_object_forms() {
    // source: 021
    let mut test = TestParser::new_with_language("021", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let error = parser.eat_key_with_span().unwrap_err();

    // 021
    assert_eq!(parser.get_span_str(error.leaf_span()), "021");
}

/// Parse computed keys with ternaries.
#[test]
fn test_parse_key_computed_ternary() {
    let mut test = TestParser::new_with_language(
        "[hasCjsFormat ? 'module' : 'import']",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();

    let (key, _span) = parser.eat_key_with_span().unwrap();

    assert!(parser.errors.is_empty(), "{:#?}", parser.errors);
    assert!(matches!(key, Key::Expression(_)));
    assert_node!(parser.tree, match key { Key::Expression(key) => key, _ => unreachable!() }, Expression::If { form, condition, then_expression, else_expression } => {
        assert_eq!(*form, IfForm::Ternary);
        assert_node!(condition, IfCondition::Expression { condition } => {
            assert_expression_path!(parser, parser.tree.get(*condition), "hasCjsFormat");
        });
        assert_node!(parser.tree, *then_expression, Expression::ScalarLiteral(ScalarLiteral::String(string)) => {
            assert_string!(parser, *string, "module");
        });
        assert_node!(parser.tree, else_expression.expect("expected else"), Expression::ScalarLiteral(ScalarLiteral::String(string)) => {
            assert_string!(parser, *string, "import");
        });
    });
}
