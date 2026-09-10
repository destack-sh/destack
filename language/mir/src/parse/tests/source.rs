use crate::{Block, Function, Global, Local, Mutability, Space, TypeDeclaration, assert_node};
use destack_source::{NodeSpanList, NodeSpanRegion, NodeSpanType};

use super::{TestParser, span_for_text, span_for_text_in, span_for_text_in_after};

/// Parsed MIR assigns exact storage to constants and globals.
#[test]
fn test_parse_global_storage() {
    let source = r#"
constant bytes: [uint8; 4] = b"data"
readonly global localValue: int32 = 1
shared global sharedValue: int32 = 2
"#;

    let (tree, _) = TestParser::new(source).parse();
    let globals = tree
        .iter_nodes::<Global>()
        .map(|(_, global)| (global.space, global.mutability))
        .collect::<Vec<_>>();

    assert_eq!(
        globals,
        vec![
            (Space::Constant, Mutability::Immutable),
            (Space::Local, Mutability::Immutable),
            (Space::Shared, Mutability::Mutable),
        ]
    );
}

/// Parsed MIR retains direct nominal heritage on each type declaration.
#[test]
fn test_parse_type_heritage() {
    let source = r#"
type Parent { }
type Base { }
type Left { }
type Right { }
type Child extends Parent, Base implements Left, Right { }
"#;

    let (tree, _) = TestParser::new(source).parse();
    let declarations = tree
        .iter_nodes::<TypeDeclaration>()
        .map(|(_, declaration)| declaration)
        .collect::<Vec<_>>();

    let child = declarations[4];
    assert_eq!(
        child.heritage.extends,
        vec![
            tree.identified_type(declarations[0].symbol).unwrap(),
            tree.identified_type(declarations[1].symbol).unwrap()
        ]
    );
    assert_eq!(
        child.heritage.implements,
        vec![
            tree.identified_type(declarations[2].symbol).unwrap(),
            tree.identified_type(declarations[3].symbol).unwrap()
        ]
    );
}

/// Parsed MIR records main spans for item and block names.
#[test]
fn test_parse_named_node_spans() {
    let source = r#"
type Callable = () => void;

readonly global Count: int32 = 1

function use(): void {
entry:
    return
}
"#;

    let (tree, _) = TestParser::new(source).parse();

    let (type_declaration_id, _) = tree.iter_nodes::<TypeDeclaration>().next().unwrap();
    let (global_id, _) = tree.iter_nodes::<Global>().next().unwrap();
    let (function_id, function) = tree.iter_nodes::<Function>().next().unwrap();
    let block_id = function.block(0);

    assert_eq!(
        tree.get_main_span(type_declaration_id),
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
        Some(span_for_text(source, "entry"))
    );

    assert_eq!(
        tree.get_side_span(
            type_declaration_id,
            NodeSpanType::Region(NodeSpanRegion::Type)
        ),
        Some(span_for_text(source, "() => void"))
    );
    assert_eq!(
        tree.get_side_span(global_id, NodeSpanType::Region(NodeSpanRegion::Type)),
        Some(span_for_text_in(
            source,
            "readonly global Count: int32 = 1",
            "int32"
        ))
    );
    assert_eq!(
        tree.get_side_span(function_id, NodeSpanType::Region(NodeSpanRegion::Type)),
        Some(span_for_text(source, "(): void"))
    );

    assert_eq!(
        tree.get_span(type_declaration_id),
        Some(span_for_text(source, "type Callable = () => void"))
    );
    assert_eq!(
        tree.get_span(global_id),
        Some(span_for_text(source, "readonly global Count: int32 = 1"))
    );
    assert_eq!(
        tree.get_span(function_id),
        Some(span_for_text(
            source,
            "function use(): void {\nentry:\n    return\n}"
        ))
    );
    assert_eq!(
        tree.get_span(block_id),
        Some(span_for_text(source, "entry:\n    return"))
    );
}

/// Parsed MIR records instruction ownership and side spans.
#[test]
fn test_parse_instruction_spans() {
    let source = r#"
function use(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = add v0, v0
    return v1
}
"#;

    let (tree, _) = TestParser::new(source).parse();

    let (_, function) = tree.iter_nodes::<Function>().next().unwrap();

    assert_node!(tree, function.block(0), Block { instructions, terminator, .. } => {
        let instruction_id = instructions[0];

        assert_eq!(
            tree.get_span(instruction_id),
            Some(span_for_text(
                source,
                "v1: int32 = add v0, v0"
            ))
        );
        assert_eq!(
            tree.get_main_span(instruction_id),
            Some(span_for_text(source, "v1"))
        );
        assert_eq!(
            tree.get_side_span(instruction_id, NodeSpanType::Region(NodeSpanRegion::Type)),
            Some(span_for_text_in(
                source,
                "v1: int32 = add v0, v0",
                "int32"
            ))
        );
        assert_eq!(
            tree.get_side_span(instruction_id, NodeSpanType::ListItem(NodeSpanList::Segment, 0)),
            Some(span_for_text_in(
                source,
                "v1: int32 = add v0, v0",
                "add"
            ))
        );
        assert_eq!(
            tree.get_side_span(instruction_id, NodeSpanType::ListItem(NodeSpanList::Segment, 1)),
            Some(span_for_text_in(
                source,
                "v1: int32 = add v0, v0",
                "v0"
            ))
        );
        assert_eq!(
            tree.get_side_span(instruction_id, NodeSpanType::ListItem(NodeSpanList::Segment, 2)),
            Some(span_for_text_in_after(
                source,
                "v1: int32 = add v0, v0",
                "v0",
                1
            ))
        );

        assert_eq!(
            tree.get_span(*terminator),
            Some(span_for_text(source, "return v1"))
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
    local l0: int32

entry:
    return
}
"#;

    let (tree, _) = TestParser::new(source).parse();
    let (local_id, _) = tree.iter_nodes::<Local>().next().unwrap();

    assert_eq!(
        tree.get_span(local_id),
        Some(span_for_text(source, "local l0: int32"))
    );
    assert_eq!(
        tree.get_main_span(local_id),
        Some(span_for_text(source, "l0"))
    );
    assert_eq!(
        tree.get_side_span(local_id, NodeSpanType::Region(NodeSpanRegion::Type)),
        Some(span_for_text_in(source, "local l0: int32", "int32"))
    );
}
