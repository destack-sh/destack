use crate::{
    Block, Field, Function, Global, Instruction, Local, Terminator, Type, TypeAlias, TypeReference,
    assert_node,
};
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

/// Recovering parse keeps later instructions in the same block after a broken instruction line.
#[test]
fn test_parse_recovers_after_instruction_error_in_same_block() {
    let source = r#"
function broken(): int32 {
b0:
    value0: int32 = int.add
    value1: int32 = 1int32
    return value1
}
"#;

    // parse
    let (tree, diagnostics) = TestParser::new(source).parse_with_diagnostics();
    let (_, function) = tree.iter_nodes::<Function>().next().unwrap();

    // diagnostic and block
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error));
    assert_eq!(function.blocks.len(), 1);

    // recovered instructions
    assert_node!(tree, function.blocks[0], Block { instructions, terminator, .. } => {
        assert_eq!(instructions.len(), 2);
        assert_node!(tree, instructions[0], Instruction::Error);
        assert!(!matches!(tree.get(instructions[1]), Instruction::Error));
        assert_node!(tree, *terminator, Terminator::Return { .. });
    });
}

/// Recovering parse keeps later instructions after a missing destination type.
#[test]
fn test_parse_recovers_after_instruction_type_hole() {
    let source = r#"
function broken(): int32 {
b0:
    v0:
    value1: int32 = 1int32
    return value1
}
"#;

    // parse
    let (tree, diagnostics) = TestParser::new(source).parse_with_diagnostics();
    let (_, function) = tree.iter_nodes::<Function>().next().unwrap();

    // diagnostic and block
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error));

    // recovered instructions
    assert_node!(tree, function.blocks[0], Block { instructions, terminator, .. } => {
        assert_eq!(instructions.len(), 2);
        assert_node!(tree, instructions[0], Instruction::Error);
        assert!(!matches!(tree.get(instructions[1]), Instruction::Error));
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

/// Recovering parse keeps the function body after a broken local declaration.
#[test]
fn test_parse_recovers_after_local_error() {
    let source = r#"
function broken(): void {
    local local0:

b0:
    return
}
"#;

    // parse
    let (tree, diagnostics) = TestParser::new(source).parse_with_diagnostics();
    let (_, function) = tree.iter_nodes::<Function>().next().unwrap();

    // diagnostic and body
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error));
    assert_eq!(function.locals.len(), 1);
    assert_eq!(function.blocks.len(), 1);

    // recovered local
    assert_node!(tree, function.locals[0], Local { ty, .. } => {
        assert_eq!(*ty, TypeReference::Missing);
    });

    // recovered terminator
    assert_node!(tree, function.blocks[0], Block { terminator, .. } => {
        assert_node!(tree, *terminator, Terminator::Return { .. });
    });
}

/// Recovering parse keeps later items after a broken type declaration.
#[test]
fn test_parse_recovers_after_type_error() {
    let source = r#"
type Broken {
    value int32
}

type Later = int32

function later(): void {
b0:
    return
}
"#;

    // parse
    let (tree, diagnostics) = TestParser::new(source).parse_with_diagnostics();

    // diagnostic and later items
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error));
    assert_eq!(tree.iter_nodes::<TypeAlias>().count(), 1);
    assert_eq!(tree.iter_nodes::<Function>().count(), 1);
}

/// Recovering parse restores lifetime names after a broken type declaration.
#[test]
fn test_parse_restores_lifetime_scope_after_type_error() {
    let source = r#"
type Broken<L: lifetime> = ref<int32, borrowed, lifetime(Missing)>

type Later = ref<int32, borrowed, lifetime(L)>

function later(): void {
b0:
    return
}
"#;

    // parse
    let (tree, diagnostics) = TestParser::new(source).parse_with_diagnostics();

    // both type declarations fail independently
    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error));
    assert_eq!(tree.iter_nodes::<TypeAlias>().count(), 0);
    assert_eq!(tree.iter_nodes::<Function>().count(), 1);
}

/// Recovering parse keeps struct fields around a missing field type.
#[test]
fn test_parse_recovers_after_field_type_hole() {
    let source = r#"
type Pair {
    x:
    y: int32
}
"#;

    // parse
    let (tree, diagnostics) = TestParser::new(source).parse_with_diagnostics();
    let (_, alias) = tree.iter_nodes::<TypeAlias>().next().unwrap();

    // diagnostic and alias
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error));

    // recovered fields
    let ty = alias.ty.ty().expect("expected recovered type alias");
    assert_node!(tree, ty, Type::Struct { fields, .. } => {
        assert_eq!(fields.len(), 2);
        assert_node!(tree, fields[0], Field { ty, .. } => {
            assert_eq!(*ty, TypeReference::Missing);
        });
        assert_node!(tree, fields[1], Field { ty, .. } => {
            assert!(ty.ty().is_some());
        });
    });
}

/// Recovering parse keeps a global declaration with a missing type.
#[test]
fn test_parse_recovers_after_global_type_hole() {
    let source = r#"
global Count:

function later(): void {
b0:
    return
}
"#;

    // parse
    let (tree, diagnostics) = TestParser::new(source).parse_with_diagnostics();
    let (_, global) = tree.iter_nodes::<Global>().next().unwrap();

    // diagnostic and later item
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error));
    assert_eq!(global.ty, TypeReference::Missing);
    assert_eq!(tree.iter_nodes::<Function>().count(), 1);
}

/// Recovering parse keeps a function with a missing parameter type.
#[test]
fn test_parse_recovers_after_parameter_type_hole() {
    let source = r#"
function broken(value0: ): void {
b0:
    return
}
"#;

    // parse
    let (tree, diagnostics) = TestParser::new(source).parse_with_diagnostics();
    let (_, function) = tree.iter_nodes::<Function>().next().unwrap();

    // diagnostic and signature
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error));
    assert_eq!(function.parameters.len(), 1);
    assert_eq!(function.parameters[0].ty, TypeReference::Missing);
}

/// Recovering parse keeps a function with a missing return type.
#[test]
fn test_parse_recovers_after_return_type_hole() {
    let source = r#"
function broken(): {
b0:
    return
}
"#;

    // parse
    let (tree, diagnostics) = TestParser::new(source).parse_with_diagnostics();
    let (_, function) = tree.iter_nodes::<Function>().next().unwrap();

    // diagnostic and body
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error));
    assert_eq!(function.return_type, TypeReference::Missing);
    assert_eq!(function.blocks.len(), 1);
}

/// Recovering parse collects multiple same-block instruction errors.
#[test]
fn test_parse_collects_multiple_instruction_errors() {
    let source = r#"
function broken(): int32 {
b0:
    value0: int32 = int.add
    value1: int32 = int.sub
    value2: int32 = 1int32
    return value2
}
"#;

    // parse
    let (tree, diagnostics) = TestParser::new(source).parse_with_diagnostics();
    let (_, function) = tree.iter_nodes::<Function>().next().unwrap();

    // diagnostics and block
    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error));

    // recovered instructions
    assert_node!(tree, function.blocks[0], Block { instructions, terminator, .. } => {
        assert_eq!(instructions.len(), 3);
        assert_node!(tree, instructions[0], Instruction::Error);
        assert_node!(tree, instructions[1], Instruction::Error);
        assert!(!matches!(tree.get(instructions[2]), Instruction::Error));
        assert_node!(tree, *terminator, Terminator::Return { .. });
    });
}
