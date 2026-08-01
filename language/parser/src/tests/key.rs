use crate::tests::TestParser;
use destack_dir::{Expression, IfForm, Key, Name, ScalarLiteral};

use crate::{assert_expression_path, assert_node, assert_string};

#[test]
fn test_report_key_named_type_expression_with_multiline_type() {
    let test = TestParser::new(
        r#"[key:
    | string
    | number]"#,
    );
    let mut parser = test.prepare();
    let error = parser.eat_key_with_range(Default::default()).unwrap_err();
    assert_eq!(parser.range_str(error.range()), ":");
}

#[test]
fn test_report_key_named_type_expression_with_newlines_before_colon_and_close_bracket() {
    let test = TestParser::new(
        r#"[key
:
string
]"#,
    );
    let mut parser = test.prepare();
    let error = parser.eat_key_with_range(Default::default()).unwrap_err();
    assert_eq!(parser.range_str(error.range()), ":");
}

/// Report computed keys with comma expressions.
#[test]
fn test_report_key_computed_comma_expression() {
    // source: [a,b]
    let test = TestParser::new("[a,b]");
    let mut parser = test.prepare();
    let error = parser.eat_key_with_range(Default::default()).unwrap_err();

    // ,
    assert_eq!(parser.range_str(error.range()), ",");
}

/// Report legacy octal numeric keys.
#[test]
fn test_report_legacy_octal_numeric_key() {
    // source: 021
    let test = TestParser::new("021");
    let mut parser = test.prepare();
    let error = parser.eat_key_with_range(Default::default()).unwrap_err();

    // 021
    assert_eq!(parser.range_str(error.range()), "021");
}

/// Parse finite integer property keys as static index keys.
#[test]
fn test_parse_key_integer_index() {
    let test = TestParser::new("2");
    let mut parser = test.prepare();
    let (key, _span) = parser.eat_key_with_range(Default::default()).unwrap();

    assert_eq!(key, Key::Name(Name::Index(2)));
    test.assert_no_errors(&parser);
}

/// Parse computed keys with ternaries.
#[test]
fn test_parse_key_computed_ternary() {
    let test = TestParser::new(r#"[hasCjsFormat ? "module" : "import"]"#);
    let mut parser = test.prepare();

    let (key, _span) = parser.eat_key_with_range(Default::default()).unwrap();

    assert!(parser.errors.is_empty(), "{:#?}", parser.errors);
    assert!(matches!(key, Key::Expression(_)));
    assert_node!(parser.tree, match key { Key::Expression(key) => key, _ => unreachable!() }, Expression::If { form, condition, then_expression, else_expression } => {
        assert_eq!(*form, IfForm::Ternary);
        let condition = condition.as_expression().expect("expected expression condition");
        assert_expression_path!(parser, parser.tree.get(condition), "hasCjsFormat");
        assert_node!(parser.tree, *then_expression, Expression::ScalarLiteral(ScalarLiteral::String(string)) => {
            assert_string!(parser, *string, "module");
        });
        assert_node!(parser.tree, else_expression.expect("expected else"), Expression::ScalarLiteral(ScalarLiteral::String(string)) => {
            assert_string!(parser, *string, "import");
        });
    });
}
