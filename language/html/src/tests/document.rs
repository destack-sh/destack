use crate::{Content, Document, assert_node};

use super::TestParser;

/// Return one 1-based line number for one source offset.
fn line_number_for_offset(source: &str, offset: u32) -> u64 {
    let line_breaks = source[..offset as usize]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count() as u64;

    line_breaks + 1
}

/// Handle long template chains without blowing up the parser path.
#[test]
fn test_parse_many_templates() {
    let test = TestParser::new();
    let source = "<template>".repeat(256);
    let (_tree, _document) = test.parse_document(&source);
}

/// Track authored element line numbers through spans.
#[test]
fn test_parse_tracks_element_lines() {
    let test = TestParser::new();
    let source = "<a>\n</a>\n<b>\n</b>";
    let (tree, document) = test.parse_document(source);

    assert_node!(tree, document, Document { children, .. } => {
        let lines = children
            .iter()
            .filter_map(|child| match tree.get(*child) {
                Content::Element(element) => Some((
                    tree.string(element.name.local).to_string(),
                    line_number_for_offset(source, tree.span(*child).start),
                )),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(lines, vec![("a".to_string(), 1), ("b".to_string(), 3)]);
    });
}
