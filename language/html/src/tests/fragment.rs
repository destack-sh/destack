use crate::parse::parser::ParseContextName;
use crate::{Fragment, Namespace, assert_node};

use super::TestParser;

/// Parse one fragment directly into one fragment root.
#[test]
fn test_parse_fragment_returns_fragment_children() {
    let test = TestParser::new();
    let context = ParseContextName {
        prefix: None,
        namespace: Namespace::Html,
        local: "div".to_string(),
    };
    let (tree, fragment) = test.parse_fragment("<span>a</span>", &context);

    assert_node!(tree, fragment, Fragment { children } => {
        assert_eq!(children.len(), 1);
    });
}
