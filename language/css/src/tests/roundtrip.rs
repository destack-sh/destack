use crate::print::print_stylesheet;
use crate::{Rule, Stylesheet, assert_node};

use super::TestParser;

/// Preserve one parsed stylesheet root and authored rule headers.
#[test]
fn test_parse_stylesheet_root() {
    let test = TestParser::new();
    let source = r#"@import "./base.css" layer(theme);@media screen{.button{color:red;&:hover{color:blue}}}"#;
    let (tree, stylesheet) = test.parse_stylesheet(source);

    assert_node!(tree, stylesheet, Stylesheet { rules, .. } => {
        assert_eq!(rules.len(), 2);

        assert_node!(tree, rules[0], Rule::Import(import_rule) => {
            assert_eq!(import_rule.url, "./base.css");
            assert_eq!(
                import_rule
                    .layer
                    .as_ref()
                    .and_then(|layer| layer.name.as_ref())
                    .map(|name| name.names.join(".")),
                Some("theme".to_string())
            );
        });
    });
}

/// Preserve selector-taking pseudo arguments through parse and print.
#[test]
fn test_roundtrip_selector_argument_pseudo_selectors() {
    let test = TestParser::new();
    let source = r#".root:local(.button,.button:hover)::cue(.caption){color:red}"#;
    let (tree, stylesheet) = test.parse_stylesheet(source);

    assert_eq!(
        print_stylesheet(&tree, stylesheet),
        ":local(.button,.button:hover).root::cue(.caption){color:red}"
    );
}

/// Preserve authored declaration order around interleaved `!important`.
#[test]
fn test_roundtrip_preserves_interleaved_important_declaration_order() {
    let test = TestParser::new();
    let source = r#".root{color:red!important;background:blue;border:1px solid red!important}"#;
    let (tree, stylesheet) = test.parse_stylesheet(source);

    assert_eq!(
        print_stylesheet(&tree, stylesheet),
        ".root{color:red!important;background:blue;border:1px solid red!important}"
    );
}

/// Preserve authored declarations in `@page` blocks with nested margin rules.
#[test]
fn test_roundtrip_preserves_page_declarations_with_nested_margin_rules() {
    let test = TestParser::new();
    let source = r#"@page :left{size:a4;margin:1cm;@top-left{content:"x";color:red!important}}"#;
    let (tree, stylesheet) = test.parse_stylesheet(source);

    assert_eq!(
        print_stylesheet(&tree, stylesheet),
        "@page :left{size:a4;margin:1cm;@top-left{content:\"x\";color:red!important}}"
    );
}
