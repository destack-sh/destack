use crate::{Function, Instruction, Terminator};
use destack_source::DiagnosticSeverity;

use super::tests::parse_fixture_source;

/// Recovering parse returns partial MIR and shared diagnostics after a syntax error.
#[test]
fn test_parse_with_diagnostics_collects_diagnostics() {
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

    let (tree, diagnostics) = parse_fixture_source(source);

    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error));

    let diagnostics = diagnostics.iter();
    let diagnostic = diagnostics.first().unwrap();
    let length = diagnostic
        .primary_span
        .span
        .end
        .saturating_sub(diagnostic.primary_span.span.start);

    assert_eq!(length, "int32".len() as u32);
    assert_eq!(tree.iter_nodes::<Function>().count(), 2);
}

/// Recovering parse keeps later blocks after a broken instruction line.
#[test]
fn test_parse_with_diagnostics_keeps_later_blocks_after_instruction_error() {
    let source = r#"
function broken(): void {
b0:
    value0: int32 = int.add

b1:
    return
}
"#;

    let (tree, diagnostics) = parse_fixture_source(source);
    let (_, function) = tree.iter_nodes::<Function>().next().unwrap();

    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error));
    assert_eq!(function.blocks.len(), 2);

    let first_block = tree.get(function.blocks[0]);
    assert!(matches!(
        tree.get(first_block.terminator),
        Terminator::Error
    ));
    assert_eq!(first_block.instructions.len(), 1);
    assert!(matches!(
        tree.get(first_block.instructions[0]),
        Instruction::Error
    ));

    let second_block = tree.get(function.blocks[1]);
    assert!(matches!(
        tree.get(second_block.terminator),
        Terminator::Return { .. }
    ));
}

/// Recovering parse keeps later blocks after a broken terminator line.
#[test]
fn test_parse_with_diagnostics_keeps_later_blocks_after_terminator_error() {
    let source = r#"
function broken(): void {
b0:
    check

b1:
    return
}
"#;

    let (tree, diagnostics) = parse_fixture_source(source);
    let (_, function) = tree.iter_nodes::<Function>().next().unwrap();

    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error));
    assert_eq!(function.blocks.len(), 2);

    let first_block = tree.get(function.blocks[0]);
    assert!(matches!(
        tree.get(first_block.terminator),
        Terminator::Error
    ));

    let second_block = tree.get(function.blocks[1]);
    assert!(matches!(
        tree.get(second_block.terminator),
        Terminator::Return { .. }
    ));
}
