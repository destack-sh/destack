use crate::print::print_document;

use super::TestParser;

/// Print one parsed document back as authored HTML.
#[test]
fn test_print_document() {
    let test = TestParser::new();
    let source = "<div class=test><span>hi</span><!--x--></div>";
    let (tree, document) = test.parse_document(source);

    assert_eq!(
        print_document(&tree, document),
        "<div class=test><span>hi</span><!--x--></div>"
    );
}
