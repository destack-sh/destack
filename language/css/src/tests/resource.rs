use crate::{
    ComponentValue, Declaration, DeclarationBlock, ImportResource, Rule, Stylesheet, Token,
    UrlResource, assert_node,
};

use super::TestParser;

/// Classify `@import` rules into structured import resources.
#[test]
fn test_parse_stylesheet_classifies_import_resources() {
    let test = TestParser::new();
    let source = r#"
        @import "./reset.css?inline";
    "#;
    let (tree, stylesheet) = test.parse_stylesheet(source);

    assert_node!(tree, stylesheet, Stylesheet { rules, .. } => {
        assert_eq!(rules.len(), 1);

        assert_node!(tree, rules[0], Rule::Import(rule) => {
            assert_eq!(rule.url, "./reset.css?inline");
            assert_eq!(
                rule.resource,
                Some(ImportResource {
                    id: 0,
                    path: "./reset.css".to_string(),
                    suffix: "?inline".to_string(),
                    is_external: false,
                })
            );
        });
    });
}

/// Classify `url(...)` declarations into structured url resources.
#[test]
fn test_parse_stylesheet_classifies_url_function_resources() {
    let test = TestParser::new();
    let source = r#"
        .hero {
            background-image: url("../images/pattern.svg#hero");
        }
    "#;
    let (tree, stylesheet) = test.parse_stylesheet(source);

    assert_node!(tree, stylesheet, Stylesheet { rules, .. } => {
        assert_eq!(rules.len(), 1);

        assert_node!(tree, rules[0], Rule::Style(rule) => {
            let declarations = rule.declarations.unwrap_or_else(|| panic!("missing declarations"));

            assert_node!(tree, declarations, DeclarationBlock { declarations } => {
                assert_eq!(declarations.len(), 1);

                assert_node!(tree, declarations[0], Declaration { value, .. } => {
                    let url_function = value
                        .components()
                        .values
                        .iter()
                        .find_map(|value| match value {
                            ComponentValue::Function(function) if function.name_eq(&tree.strings, "url") => {
                                Some(function)
                            }
                            _ => None,
                        })
                        .unwrap_or_else(|| panic!("missing url function"));

                    assert_eq!(
                        url_function.url_resource,
                        Some(UrlResource {
                            id: 0,
                            path: "../images/pattern.svg".to_string(),
                            suffix: "#hero".to_string(),
                            is_external: false,
                        })
                    );

                    assert_node!(&url_function.arguments.values[0], ComponentValue::Token(Token::String(value)) => {
                        assert_eq!(value, "../images/pattern.svg#hero");
                    });
                });
            });
        });
    });
}
