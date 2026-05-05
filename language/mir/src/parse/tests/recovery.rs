use crate::{Block, Function, Instruction, Terminator, assert_node};
use destack_source::DiagnosticSeverity;

use super::tests::TestParser;

/// Recovering parse returns partial MIR and shared diagnostics after a syntax error.
#[test]
fn test_parse_collects_diagnostics() {
    let source = r#"
function good(): void {
b0:
    return
}

global Broken int32 = 0int32

function later(): void {
b0:
    return
}
"#;

    // parse
    let (tree, diagnostics) = TestParser::new(source).parse_with_diagnostics();

    // diagnostic and later function
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error));

    // int32
    let diagnostic = diagnostics.iter().next().unwrap();
    let primary_span = diagnostic.primary_label().span;
    let length = primary_span.end.saturating_sub(primary_span.start);

    assert_eq!(length, "int32".len() as u32);
    assert_eq!(tree.iter_nodes::<Function>().count(), 2);
}

/// Recovering parse keeps later blocks after a broken instruction line.
#[test]
fn test_parse_recovers_after_instruction_error() {
    let source = r#"
function broken(): void {
b0:
    value0: int32 = int.add

b1:
    return
}
"#;

    // parse
    let (tree, diagnostics) = TestParser::new(source).parse_with_diagnostics();
    let (_, function) = tree.iter_nodes::<Function>().next().unwrap();

    // diagnostic and blocks
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error));
    assert_eq!(function.blocks.len(), 2);

    // b0
    assert_node!(tree, function.blocks[0], Block { instructions, terminator, .. } => {
        assert_eq!(instructions.len(), 1);
        assert_node!(tree, instructions[0], Instruction::Error);
        assert_node!(tree, *terminator, Terminator::Error);
    });

    // b1
    assert_node!(tree, function.blocks[1], Block { terminator, .. } => {
        assert_node!(tree, *terminator, Terminator::Return { .. });
    });
}

/// Recovering parse keeps later blocks after a broken terminator line.
#[test]
fn test_parse_recovers_after_terminator_error() {
    let source = r#"
function broken(): void {
b0:
    check

b1:
    return
}
"#;

    // parse
    let (tree, diagnostics) = TestParser::new(source).parse_with_diagnostics();
    let (_, function) = tree.iter_nodes::<Function>().next().unwrap();

    // diagnostic and blocks
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error));
    assert_eq!(function.blocks.len(), 2);

    // b0
    assert_node!(tree, function.blocks[0], Block { terminator, .. } => {
        assert_node!(tree, *terminator, Terminator::Error);
    });

    // b1
    assert_node!(tree, function.blocks[1], Block { terminator, .. } => {
        assert_node!(tree, *terminator, Terminator::Return { .. });
    });
}
