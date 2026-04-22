use crate::{Block, Function, Global, Local, TypeAlias, TypeReference, assert_node};
use destack_source::{NodeSpanList, NodeSpanRegion, NodeSpanType};

use super::tests::{TestParser, span_for_text, span_for_text_in, span_for_text_in_after};

/// Parsed MIR records main spans for item and block names.
#[test]
fn test_parse_named_node_spans() {
    let source = r#"
type Callable = () => void

global Count: int32, readonly = 1int32

function use(): void {
entry0:
    return
}
"#;

    let (tree, _) = TestParser::new(source).parse();

    let (type_alias_id, _) = tree.iter_nodes::<TypeAlias>().next().unwrap();
    let (global_id, _) = tree.iter_nodes::<Global>().next().unwrap();
    let (function_id, function) = tree.iter_nodes::<Function>().next().unwrap();
    let block_id = function.blocks[0];

    assert_eq!(
        tree.get_main_span(type_alias_id),
        Some(span_for_text(source, "Callable"))
    );
    assert_eq!(
        tree.get_main_span(global_id),
        Some(span_for_text(source, "Count"))
    );
    assert_eq!(
        tree.get_main_span(function_id),
        Some(span_for_text(source, "use"))
    );
    assert_eq!(
        tree.get_main_span(block_id),
        Some(span_for_text(source, "entry0"))
    );

    assert_eq!(
        tree.get_side_span(type_alias_id, NodeSpanType::Region(NodeSpanRegion::Type)),
        Some(span_for_text(source, "() => void"))
    );
    assert_eq!(
        tree.get_side_span(global_id, NodeSpanType::Region(NodeSpanRegion::Type)),
        Some(span_for_text_in(
            source,
            "global Count: int32, readonly = 1int32",
            "int32"
        ))
    );
    assert_eq!(
        tree.get_side_span(function_id, NodeSpanType::Region(NodeSpanRegion::Type)),
        Some(span_for_text(source, "(): void"))
    );

    assert_eq!(
        tree.get_span(type_alias_id),
        Some(span_for_text(source, "type Callable = () => void"))
    );
    assert_eq!(
        tree.get_span(global_id),
        Some(span_for_text(
            source,
            "global Count: int32, readonly = 1int32"
        ))
    );
    assert_eq!(
        tree.get_span(function_id),
        Some(span_for_text(
            source,
            "function use(): void {\nentry0:\n    return\n}"
        ))
    );
    assert_eq!(
        tree.get_span(block_id),
        Some(span_for_text(source, "entry0:\n    return"))
    );
}

/// Parsed MIR records instruction ownership and side spans.
#[test]
fn test_parse_instruction_spans() {
    let source = r#"
function use(input0: int32): int32 {
entry0(input0: int32):
    result1: int32 = int.add input0, input0
    return result1
}
"#;

    let (tree, _) = TestParser::new(source).parse();

    let (_, function) = tree.iter_nodes::<Function>().next().unwrap();

    assert_node!(tree, function.blocks[0], Block { instructions, terminator, .. } => {
        let instruction_id = instructions[0];

        assert_eq!(
            tree.get_span(instruction_id),
            Some(span_for_text(
                source,
                "result1: int32 = int.add input0, input0"
            ))
        );
        assert_eq!(
            tree.get_main_span(instruction_id),
            Some(span_for_text(source, "result1"))
        );
        assert_eq!(
            tree.get_side_span(instruction_id, NodeSpanType::Region(NodeSpanRegion::Type)),
            Some(span_for_text_in(
                source,
                "result1: int32 = int.add input0, input0",
                "int32"
            ))
        );
        assert_eq!(
            tree.get_side_span(instruction_id, NodeSpanType::ListItem(NodeSpanList::Segment, 0)),
            Some(span_for_text_in(
                source,
                "result1: int32 = int.add input0, input0",
                "int.add"
            ))
        );
        assert_eq!(
            tree.get_side_span(instruction_id, NodeSpanType::ListItem(NodeSpanList::Segment, 1)),
            Some(span_for_text_in(
                source,
                "result1: int32 = int.add input0, input0",
                "input0"
            ))
        );
        assert_eq!(
            tree.get_side_span(instruction_id, NodeSpanType::ListItem(NodeSpanList::Segment, 2)),
            Some(span_for_text_in_after(
                source,
                "result1: int32 = int.add input0, input0",
                "input0",
                1
            ))
        );

        assert_eq!(
            tree.get_span(*terminator),
            Some(span_for_text(source, "return result1"))
        );
        assert_eq!(
            tree.get_main_span(*terminator),
            Some(span_for_text(source, "return"))
        );
    });
}

/// Parsed MIR records local declaration spans.
#[test]
fn test_parse_local_spans() {
    let source = r#"
function use(): void {
    local local0: int32

entry0:
    return
}
"#;

    let (tree, _) = TestParser::new(source).parse();
    let (local_id, _) = tree.iter_nodes::<Local>().next().unwrap();

    assert_eq!(
        tree.get_span(local_id),
        Some(span_for_text(source, "local local0: int32"))
    );
    assert_eq!(
        tree.get_main_span(local_id),
        Some(span_for_text(source, "local0"))
    );
    assert_eq!(
        tree.get_side_span(local_id, NodeSpanType::Region(NodeSpanRegion::Type)),
        Some(span_for_text_in(source, "local local0: int32", "int32"))
    );
}

/// Parsing a raw alias named String does not implicitly bless a well known string type.
#[test]
fn test_parse_string_alias_does_not_mark_well_known_string_type() {
    let source = r#"
type String {
    lengthUtf16: uint32;
    lengthBytes: uint32;
    hash: uint64;
    flags: uint32;
    data: ref<uint8, raw>;
}"#;

    let (tree, _) = TestParser::new(source).parse();

    assert_eq!(tree.string_type(), None);
    assert_eq!(tree.string_layout_id(), None);
}

#[test]
fn test_type_alias_carries_layout_metadata() {
    let source = r#"
type Env {
    value: int32;
}

function makeEnv(): ref<Env, managed> {
b0:
    v0: ref<Env, managed> = new Env
    return v0
}"#;

    let (tree, _) = TestParser::new(source).parse();
    let alias = tree
        .iter_nodes::<TypeAlias>()
        .next()
        .map(|(_, alias)| alias)
        .expect("missing type alias");

    assert_node!(alias.ty, TypeReference::Type(alias_type) => {
        tree.type_layout_id(alias_type)
            .expect("missing type alias layout id");
        let layout = tree
            .type_layout(alias_type)
            .expect("missing type alias layout");

        assert_eq!(layout.size, 4);
        assert_eq!(layout.alignment, 4);
        assert_eq!(layout.fields.len(), 1);
    });
}
