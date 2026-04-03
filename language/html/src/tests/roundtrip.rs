use crate::print::print_document;

use super::TestParser;

/// Preserve authored attribute value forms when printing.
#[test]
fn test_roundtrip_authored_attribute_value_forms() {
    let test = TestParser::new();
    let source = r#"<div alpha='x' beta="y" gamma=z></div>"#;
    let (tree, document) = test.parse_document(source);

    assert_eq!(print_document(&tree, document), source);
}

/// Preserve authored self-closing slash spacing when printing.
#[test]
fn test_roundtrip_authored_self_closing_style() {
    let test = TestParser::new();
    let compact_source = "<div/>";
    let spaced_source = "<div />";
    let (compact_tree, compact_document) = test.parse_document(compact_source);
    let (spaced_tree, spaced_document) = test.parse_document(spaced_source);

    assert_eq!(print_document(&compact_tree, compact_document), "<div/>");
    assert_eq!(print_document(&spaced_tree, spaced_document), "<div />");
}

/// Preserve authored doctype quote styles when printing.
#[test]
fn test_roundtrip_authored_doctype_quote_styles() {
    let test = TestParser::new();
    let source = r#"<!doctype html public 'pubid' "sysid"><div></div>"#;
    let (tree, document) = test.parse_document(source);

    assert_eq!(print_document(&tree, document), source);
}

/// Avoid keeping implied HTML elements as authored nodes.
#[test]
fn test_roundtrip_flattens_implied_html_elements() {
    let test = TestParser::new();
    let source = "<table><tr><td>x</td></tr></table>";
    let (tree, document) = test.parse_document(source);

    assert_eq!(print_document(&tree, document), source);
}

/// Preserve omitted authored end tags when printing.
#[test]
fn test_roundtrip_preserves_omitted_end_tags() {
    let test = TestParser::new();
    let source = "<ul><li>a<li>b</ul>";
    let (tree, document) = test.parse_document(source);

    assert_eq!(print_document(&tree, document), source);
}

/// Continue parsing after stray non-script markup.
#[test]
fn test_parse_continues_past_non_script_markup() {
    let test = TestParser::new();
    let source = "<div>a</div><div>b</div> other stuff";
    let (tree, document) = test.parse_document(source);

    assert_eq!(print_document(&tree, document), source);
}
