use crate::parse::{ParseOptions, Parser};
use crate::{Function, Global, Local, TypeAlias};
use destack_source::{FileId, NodeSpanType};

use super::tests::{span_for_text, span_for_text_in, span_for_text_in_after};

/// Parsed MIR records main spans for item and block names.
#[test]
fn test_parse_records_main_spans_for_named_nodes() {
    let source = r#"
type Callable = closure() -> void

global Count: int32, readonly = 1int32

function use(): void {
entry0:
    return
}
"#;

    let (tree, _) = Parser::parse(FileId::new(0), source, ParseOptions::default())
        .validate()
        .expect("parse failed");

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
        tree.get_side_span(type_alias_id, NodeSpanType::Type),
        Some(span_for_text(source, "closure() -> void"))
    );
    assert_eq!(
        tree.get_side_span(global_id, NodeSpanType::Type),
        Some(span_for_text_in(
            source,
            "global Count: int32, readonly = 1int32",
            "int32"
        ))
    );
    assert_eq!(
        tree.get_side_span(function_id, NodeSpanType::Type),
        Some(span_for_text(source, "(): void"))
    );

    assert_eq!(
        tree.get_span(type_alias_id),
        Some(span_for_text(source, "type Callable = closure() -> void"))
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
fn test_parse_records_instruction_spans() {
    let source = r#"
function use(input0: int32): int32 {
entry0(input0: int32):
    result1: int32 = int.add input0, input0
    return result1
}
"#;

    let (tree, _) = Parser::parse(FileId::new(0), source, ParseOptions::default())
        .validate()
        .expect("parse failed");

    let (_, function) = tree.iter_nodes::<Function>().next().unwrap();
    let block = tree.get(function.blocks[0]);
    let instruction_id = block.instructions[0];

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
        tree.get_side_span(instruction_id, NodeSpanType::Type),
        Some(span_for_text_in(
            source,
            "result1: int32 = int.add input0, input0",
            "int32"
        ))
    );
    assert_eq!(
        tree.get_side_span(instruction_id, NodeSpanType::Segment(0)),
        Some(span_for_text_in(
            source,
            "result1: int32 = int.add input0, input0",
            "int.add"
        ))
    );
    assert_eq!(
        tree.get_side_span(instruction_id, NodeSpanType::Segment(1)),
        Some(span_for_text_in(
            source,
            "result1: int32 = int.add input0, input0",
            "input0"
        ))
    );
    assert_eq!(
        tree.get_side_span(instruction_id, NodeSpanType::Segment(2)),
        Some(span_for_text_in_after(
            source,
            "result1: int32 = int.add input0, input0",
            "input0",
            1
        ))
    );

    let terminator_id = block.terminator;

    assert_eq!(
        tree.get_span(terminator_id),
        Some(span_for_text(source, "return result1"))
    );
    assert_eq!(
        tree.get_main_span(terminator_id),
        Some(span_for_text(source, "return"))
    );
}

/// Parsed MIR records local declaration spans.
#[test]
fn test_parse_records_local_spans() {
    let source = r#"
function use(): void {
    local local0: int32

entry0:
    return
}
"#;

    let (tree, _) = Parser::parse(FileId::new(0), source, ParseOptions::default())
        .validate()
        .expect("parse failed");
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
        tree.get_side_span(local_id, NodeSpanType::Type),
        Some(span_for_text_in(source, "local local0: int32", "int32"))
    );
}

/// Parsing a raw alias named String does not implicitly bless a well known string type.
#[test]
fn test_parse_string_alias_does_not_set_well_known_string_type() {
    let source = r#"
type String {
    lengthUtf16: uint32;
    lengthBytes: uint32;
    hash: uint64;
    flags: uint32;
    data: ref<uint8, raw>;
}"#;

    let (tree, _) = Parser::parse(FileId::new(0), source, ParseOptions::default())
        .validate()
        .expect("parse failed");

    assert_eq!(tree.string_type(), None);
    assert_eq!(tree.string_layout_id(), None);
}

#[test]
fn test_type_alias_preserves_layout_metadata() {
    let source = r#"
type Env {
    value: int32;
}

function makeEnv(): ref<Env, managed> {
b0:
    v0: ref<Env, managed> = managed.alloc Env
    return v0
}"#;

    let (tree, _) = Parser::parse(FileId::new(0), source, ParseOptions::default())
        .validate()
        .expect("parse failed");
    let alias = tree
        .iter_nodes::<TypeAlias>()
        .next()
        .map(|(_, alias)| alias)
        .expect("missing type alias");

    assert!(tree.type_layout_id(alias.ty).is_some());
    assert!(tree.type_layout(alias.ty).is_some());
}
