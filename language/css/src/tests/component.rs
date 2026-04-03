use crate::print::print_component_fragment;
use crate::{BlockKind, ComponentFragment, ComponentValue, assert_node};

use super::TestParser;

/// Preserve generic CSS component values through tokenization and printing.
#[test]
fn test_roundtrip_component_value_list_source() {
    let test = TestParser::new();
    let source = r#"@media screen and (width >= 20px){.button:hover{color:red;url("/a.png")}}"#;
    let (tree, fragment) = test.parse_component_fragment(source);

    assert_eq!(print_component_fragment(&tree, fragment), source);
}

/// Preserve nested functions and blocks as structured component values.
#[test]
fn test_parse_nested_component_values() {
    let test = TestParser::new();
    let source = r#"url("/a.png") calc(100% - 1rem) [data-kind="x"]"#;
    let (tree, fragment) = test.parse_component_fragment(source);

    assert_node!(tree, fragment, ComponentFragment { value } => {
        let values = &value.values;

        assert_node!(&values[0], ComponentValue::Function(function) => {
            assert!(function.name_eq(&tree.strings, "url"));
        });

        let calc_function = values
            .iter()
            .find_map(|value| match value {
                ComponentValue::Function(function) if function.name_eq(&tree.strings, "calc") => {
                    Some(function)
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("missing calc function"));

        assert!(calc_function.name_eq(&tree.strings, "calc"));

        let square_bracket_block = values
            .iter()
            .find_map(|value| match value {
                ComponentValue::Block(block) if block.kind == BlockKind::SquareBracket => {
                    Some(block)
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("missing square bracket block"));

        assert_eq!(square_bracket_block.kind, BlockKind::SquareBracket);
    });
}
