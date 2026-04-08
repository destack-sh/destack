use crate::print::{print_component_fragment, print_stylesheet};

use super::TestParser;

/// Print compact canonical CSS for one nested media rule.
#[test]
fn test_print_stylesheet_compact() {
    let test = TestParser::new();
    let source = r#"@media screen {.button { color: red; background: blue }}"#;
    let (tree, stylesheet) = test.parse_stylesheet(source);

    assert_eq!(
        print_stylesheet(&tree, stylesheet),
        "@media screen{.button{color:red;background:blue}}"
    );
}

/// Preserve import prelude ordering while printing compact canonical CSS.
#[test]
fn test_print_stylesheet_preserves_import_prelude_order() {
    let test = TestParser::new();
    let source = r#"
@import "./base.css" layer(theme) screen;
"#;
    let (tree, stylesheet) = test.parse_stylesheet(source);

    assert_eq!(
        print_stylesheet(&tree, stylesheet),
        "@import \"./base.css\" layer(theme) screen;"
    );
}

/// Preserve supports groups while printing compact canonical CSS.
#[test]
fn test_print_stylesheet_preserves_supports_groups() {
    let test = TestParser::new();
    let source = r#"
@supports (display: grid) {
    .grid {
        display: grid;
    }
}
"#;
    let (tree, stylesheet) = test.parse_stylesheet(source);

    assert_eq!(
        print_stylesheet(&tree, stylesheet),
        "@supports (display: grid){.grid{display:grid}}"
    );
}

/// Preserve namespace rules in both string and url forms.
#[test]
fn test_print_stylesheet_preserves_namespace_forms() {
    let test = TestParser::new();
    let source = r#"
@namespace svg url("http://www.w3.org/2000/svg");
@namespace "https://example.com/default";
"#;
    let (tree, stylesheet) = test.parse_stylesheet(source);

    assert_eq!(
        print_stylesheet(&tree, stylesheet),
        "@namespace svg \"http://www.w3.org/2000/svg\";@namespace \"https://example.com/default\";"
    );
}

/// Preserve property rules as compact canonical CSS.
#[test]
fn test_print_stylesheet_preserves_property_rules() {
    let test = TestParser::new();
    let source = r#"
@property --accent {
    syntax: "<color>";
    inherits: false;
    initial-value: red;
}
"#;
    let (tree, stylesheet) = test.parse_stylesheet(source);

    assert_eq!(
        print_stylesheet(&tree, stylesheet),
        "@property --accent{syntax: <color>; inherits: false; initial-value: red;}"
    );
}

/// Preserve page rules with nested page margin rules.
#[test]
fn test_print_stylesheet_preserves_page_margin_rules() {
    let test = TestParser::new();
    let source = r#"
@page :left {
    size: a4;
    margin: 1cm;

    @top-left {
        content: "x";
        color: red !important;
    }
}
"#;
    let (tree, stylesheet) = test.parse_stylesheet(source);

    assert_eq!(
        print_stylesheet(&tree, stylesheet),
        "@page :left{size:a4;margin:1cm;@top-left{content:\"x\";color:red !important}}"
    );
}

/// Preserve selector pseudo arguments and canonical combinator spacing.
#[test]
fn test_print_stylesheet_preserves_selector_canonicalization() {
    let test = TestParser::new();
    let source = r#"
.root:where(.button,.button:hover) > [data-kind="x"] + button::before {
    color: red;
}
"#;
    let (tree, stylesheet) = test.parse_stylesheet(source);

    assert_eq!(
        print_stylesheet(&tree, stylesheet),
        ":where(.button, :hover.button).root > [data-kind=\"x\"] + button::before{color:red}"
    );
}

/// Preserve the current nesting serialization contract.
#[test]
fn test_print_stylesheet_preserves_nesting_contract() {
    let test = TestParser::new();
    let source = r#"
.root {
    color: red;

    &:hover {
        color: blue;
    }
}
"#;
    let (tree, stylesheet) = test.parse_stylesheet(source);

    assert_eq!(
        print_stylesheet(&tree, stylesheet),
        ".root{color:red;:hover&{color:blue}}"
    );
}

/// Canonicalize component tokens through the compact printer.
#[test]
fn test_print_component_fragment_canonicalizes_tokens() {
    let test = TestParser::new();
    let source = r#"url("../images/pattern.svg#hero") calc(100% - 1rem) .5em "x y""#;
    let (tree, fragment) = test.parse_component_fragment(source);

    assert_eq!(
        print_component_fragment(&tree, fragment),
        "url(\"../images/pattern.svg#hero\") calc(100% - 1rem) 0.5em \"x y\""
    );
}
