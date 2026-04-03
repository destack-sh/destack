use crate::{HtmlFormatOptions, format_document};

use super::TestParser;

/// Preserve element nesting while formatting one HTML document.
#[test]
fn test_format_document() {
    let test = TestParser::new();
    let source = "<div><span>hi</span><span>bye</span></div>";
    let (tree, document) = test.parse_document(source);
    let formatted = format_document(&tree, document, HtmlFormatOptions::default()).unwrap();

    assert_eq!(
        formatted,
        "<div>\n    <span>hi</span>\n    <span>bye</span>\n</div>\n"
    );
}
