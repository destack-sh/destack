use crate::{CssFormatOptions, format_stylesheet};

use super::TestParser;

/// Preserve block structure while formatting nested CSS rules.
#[test]
fn test_format_stylesheet() {
    let test = TestParser::new();
    let source = r#"@media screen{.button{color:red;background:blue}}"#;
    let (tree, stylesheet) = test.parse_stylesheet(source);
    let formatted = format_stylesheet(&tree, stylesheet, CssFormatOptions::default()).unwrap();

    assert_eq!(
        formatted,
        "@media screen {\n    .button {\n        color: red;\n        background: blue;\n    }\n}\n"
    );
}
