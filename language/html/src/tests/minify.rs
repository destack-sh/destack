use crate::print::{PrintOptions, print_document_with_options};

use super::TestParser;

fn print_minified(source: &str) -> String {
    let test = TestParser::new();
    let (tree, document) = test.parse_document(source);

    print_document_with_options(&tree, document, PrintOptions { is_minified: true })
}

/// Drop comments, normalize boolean attributes, and avoid self-closing syntax while minifying.
#[test]
fn test_print_document_minified_drops_comments_and_self_closing_syntax() {
    let source = r#"<div class="hero" disabled="disabled"><span>hi</span><!--x--><br /></div>"#;
    let printed = print_minified(source);

    assert_eq!(
        printed,
        "<div class=hero disabled><span>hi</span><br></div>"
    );
}

/// Collapse text whitespace and keep quotes only when attributes need them.
#[test]
fn test_print_document_minified_collapses_text_and_quotes_attributes_when_needed() {
    let source = r#"<div data-title="a b" checked="checked">  hello   world  </div>"#;
    let printed = print_minified(source);

    assert_eq!(
        printed,
        "<div data-title=\"a b\" checked> hello world </div>"
    );
}

/// Preserve raw text content while minifying surrounding html.
#[test]
fn test_print_document_minified_preserves_raw_text_content() {
    let source = r#"<script>if (a < b) console.log("ok")</script>"#;
    let printed = print_minified(source);

    assert_eq!(printed, source);
}

/// Preserve title raw-text content while minifying surrounding html.
#[test]
fn test_print_document_minified_preserves_title_raw_text() {
    let source = r#"<head><title>  keep <>& text  </title></head>"#;
    let printed = print_minified(source);

    assert_eq!(printed, source);
}

/// Preserve foreign self-closing syntax while minifying surrounding markup.
#[test]
fn test_print_document_minified_preserves_foreign_self_closing_syntax() {
    let source = r#"<svg viewBox="0 0 10 10"><path /></svg>"#;
    let printed = print_minified(source);

    assert_eq!(printed, source);
}

/// Minify html attributes without rewriting namespaced foreign attribute spelling.
#[test]
fn test_print_document_minified_preserves_foreign_attribute_names() {
    let source = r##"<svg viewBox="0 0 10 10"><use xlink:href="#icon" /></svg>"##;
    let printed = print_minified(source);

    assert_eq!(
        printed,
        "<svg viewBox=\"0 0 10 10\"><use xlink:href=#icon /></svg>"
    );
}

/// Preserve authored doctype spelling and quote style while minifying the rest of the document.
#[test]
fn test_print_document_minified_preserves_doctype_quotes() {
    let source = r#"<!doctype html public 'pubid' "sysid"><div class="hero"></div>"#;
    let printed = print_minified(source);

    assert_eq!(
        printed,
        "<!doctype html public 'pubid' \"sysid\"><div class=hero></div>"
    );
}

/// Preserve style and textarea raw-text content while minifying surrounding markup.
#[test]
fn test_print_document_minified_preserves_style_and_textarea_raw_text() {
    let source = r#"<div><style>.hero > span { color: red; }</style><textarea>  keep
  spacing  </textarea></div>"#;
    let printed = print_minified(source);

    assert_eq!(printed, source);
}

/// Drop whitespace-only text nodes while keeping inline text boundaries stable.
#[test]
fn test_print_document_minified_drops_whitespace_only_text_nodes() {
    let source = "<div> <span>alpha</span> <span>beta</span> </div>";
    let printed = print_minified(source);

    assert_eq!(printed, "<div><span>alpha</span><span>beta</span></div>");
}

/// Keep one collapsed space around inline phrasing content when authored text needed it.
#[test]
fn test_print_document_minified_preserves_inline_text_boundaries() {
    let source = "<p>alpha <span>beta</span> gamma</p>";
    let printed = print_minified(source);

    assert_eq!(printed, "<p>alpha <span>beta</span> gamma</p>");
}

/// Drop comments between adjacent text nodes without inventing extra whitespace.
#[test]
fn test_print_document_minified_drops_comments_between_text_nodes() {
    let source = "<div>alpha<!--x-->beta</div>";
    let printed = print_minified(source);

    assert_eq!(printed, "<div>alphabeta</div>");
}

/// Escape unsafe unquoted attribute characters instead of emitting invalid bare values.
#[test]
fn test_print_document_minified_escapes_unquoted_attribute_values() {
    let source = r#"<div data-token="a&b=c`d"></div>"#;
    let printed = print_minified(source);

    assert_eq!(printed, "<div data-token=\"a&amp;b=c`d\"></div>");
}

/// Keep quotes around attribute values whose whitespace makes bare serialization invalid.
#[test]
fn test_print_document_minified_keeps_quotes_for_source_set_values() {
    let source = r#"<img srcset="./hero-small.jpg 1x, ./hero-large.jpg 2x">"#;
    let printed = print_minified(source);

    assert_eq!(
        printed,
        r#"<img srcset="./hero-small.jpg 1x, ./hero-large.jpg 2x">"#
    );
}

/// Drop document-level comments while preserving the surrounding document structure.
#[test]
fn test_print_document_minified_drops_document_level_comments() {
    let source = "<!doctype html><!--x--><html><body><div>alpha</div></body></html>";
    let printed = print_minified(source);

    assert_eq!(
        printed,
        "<!doctype html><html><body><div>alpha</div></body></html>"
    );
}

/// Preserve omitted authored end tags while minifying.
#[test]
fn test_print_document_minified_preserves_omitted_end_tags() {
    let source = "<ul><li>a<li>b</ul>";
    let printed = print_minified(source);

    assert_eq!(printed, source);
}
