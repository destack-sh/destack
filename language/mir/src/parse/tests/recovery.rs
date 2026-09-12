use crate::{
    Block, Field, Function, Global, Instruction, Local, LocalNodeId, Terminator, Tree, Type,
    TypeDeclaration, assert_node,
};
use destack_source::DiagnosticSeverity;

use super::TestParser;

/// Recovering parse returns partial MIR and shared diagnostics after a syntax error.
#[test]
fn test_parse_collects_diagnostics() {
    let source = r#"
function good(): void {
b0:
    return
}

global Broken int32 = 0

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
    let primary_span = diagnostic.primary_label().target.span().unwrap();
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
    v0: int32 = add

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
    assert_eq!(function.blocks().len(), 2);

    // b0
    assert_node!(tree, function.block(0), Block { instructions, terminator, .. } => {
        assert_eq!(instructions.len(), 1);
        assert_node!(tree, instructions[0], Instruction::Error);
        assert_node!(tree, *terminator, Terminator::Error);
    });

    // b1
    assert_node!(tree, function.block(1), Block { terminator, .. } => {
        assert_node!(tree, *terminator, Terminator::Return { .. });
    });
}

/// Recovering parse keeps later instructions in the same block after a broken instruction line.
#[test]
fn test_parse_recovers_after_instruction_error_in_same_block() {
    let source = r#"
function broken(): int32 {
b0:
    v0: int32 = add
    v1: int32 = 1
    return v1
}
"#;

    // parse
    let (tree, diagnostics) = TestParser::new(source).parse_with_diagnostics();
    let (_, function) = tree.iter_nodes::<Function>().next().unwrap();

    // diagnostic and block
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error));
    assert_eq!(function.blocks().len(), 1);

    // recovered instructions
    assert_node!(tree, function.block(0), Block { instructions, terminator, .. } => {
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
    v1: int32 = 1
    return v1
}
"#;

    // parse
    let (tree, diagnostics) = TestParser::new(source).parse_with_diagnostics();
    let (_, function) = tree.iter_nodes::<Function>().next().unwrap();

    // diagnostic and block
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error));

    // recovered instructions
    assert_node!(tree, function.block(0), Block { instructions, terminator, .. } => {
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
    assert_eq!(function.blocks().len(), 2);

    // b0
    assert_node!(tree, function.block(0), Block { terminator, .. } => {
        assert_node!(tree, *terminator, Terminator::Error);
    });

    // b1
    assert_node!(tree, function.block(1), Block { terminator, .. } => {
        assert_node!(tree, *terminator, Terminator::Return { .. });
    });
}

/// Recovering parse keeps the function body after a broken local declaration.
#[test]
fn test_parse_recovers_after_local_error() {
    let source = r#"
function broken(): void {
    local l0:

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
    assert_eq!(function.locals().len(), 1);
    assert_eq!(function.blocks().len(), 1);

    // recovered local
    assert_node!(tree, function.local(0), Local { ty, .. } => {
        assert_error_type(&tree, *ty);
    });

    // recovered terminator
    assert_node!(tree, function.block(0), Block { terminator, .. } => {
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
    assert_eq!(
        tree.iter_nodes::<TypeDeclaration>()
            .filter(|(_, declaration)| declaration.name.is_some())
            .count(),
        1
    );
    assert!(
        tree.iter_nodes::<TypeDeclaration>()
            .filter(|(_, declaration)| declaration.name.is_none())
            .all(|(_, declaration)| declaration.definition.is_none())
    );
    assert_eq!(tree.iter_nodes::<Function>().count(), 1);
}

/// Require declared representations instead of transparent aliases.
#[test]
fn test_parse_rejects_type_aliases() {
    for (source, definition) in [
        ("type Recursive = Recursive;", "Recursive"),
        ("type A; type Alias = A;", "A"),
        ("type A<T>; type Alias = A<int32>;", "A<int32>"),
        ("type Alias<T> = T;", "T"),
    ] {
        let (_, diagnostics) = TestParser::new(source).parse_with_diagnostics();
        let position = source.rfind(definition).unwrap();

        // report the unresolved alias at its defining type expression
        assert_eq!(diagnostics.len(), 1);
        let diagnostic = diagnostics.iter().next().unwrap();
        assert_eq!(
            diagnostic.message,
            "MIR type declaration requires a representation; resolve transparent aliases before MIR"
        );
        let span = diagnostic.primary.target.span().unwrap();
        assert_eq!(span.start as usize, position);
        assert_eq!(span.end as usize, position + definition.len());
    }
}

/// Recovering parse restores lifetime names after a broken type declaration.
#[test]
fn test_parse_restores_lifetime_scope_after_type_error() {
    let source = r#"
type Broken<'L> = ref<int32, borrowed, 'Missing & local, mutable>

type Later = ref<int32, borrowed, 'L & local, mutable>

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
    assert_eq!(
        tree.iter_nodes::<TypeDeclaration>()
            .filter(|(_, declaration)| declaration.name.is_some())
            .count(),
        0
    );
    assert!(
        tree.iter_nodes::<TypeDeclaration>()
            .filter(|(_, declaration)| declaration.name.is_none())
            .all(|(_, declaration)| declaration.definition.is_none())
    );
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
    let (_, declaration) = tree.iter_nodes::<TypeDeclaration>().next().unwrap();

    // diagnostic and declaration
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error));

    // recovered fields
    assert_node!(tree, declaration.definition.unwrap(), Type::Struct { fields, .. } => {
        assert_eq!(fields.len(), 2);
        assert_node!(tree, fields[0], Field { ty, .. } => {
            assert_error_type(&tree, *ty);
        });
        assert_node!(tree, fields[1], Field { ty, .. } => {
            assert!(!matches!(tree.get(*ty), Type::Error));
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
    assert_error_type(&tree, global.ty);
    assert_eq!(tree.iter_nodes::<Function>().count(), 1);
}

/// Recovering parse keeps a function with a missing parameter type.
#[test]
fn test_parse_recovers_after_parameter_type_hole() {
    let source = r#"
function broken(v0: ): void {
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
    assert_error_type(&tree, function.parameters[0].ty);
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
    assert_error_type(&tree, function.return_type);
    assert_eq!(function.blocks().len(), 1);
}

/// Assert one type node recovered as the error type.
fn assert_error_type(tree: &Tree, ty: LocalNodeId<Type>) {
    assert_node!(tree, ty, Type::Error);
}

/// Recovering parse collects multiple same-block instruction errors.
#[test]
fn test_parse_collects_multiple_instruction_errors() {
    let source = r#"
function broken(): int32 {
b0:
    v0: int32 = add
    v1: int32 = sub
    v2: int32 = 1
    return v2
}
"#;

    // parse
    let (tree, diagnostics) = TestParser::new(source).parse_with_diagnostics();
    let (_, function) = tree.iter_nodes::<Function>().next().unwrap();

    // diagnostics and block
    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error));

    // recovered instructions
    assert_node!(tree, function.block(0), Block { instructions, terminator, .. } => {
        assert_eq!(instructions.len(), 3);
        assert_node!(tree, instructions[0], Instruction::Error);
        assert_node!(tree, instructions[1], Instruction::Error);
        assert!(!matches!(tree.get(instructions[2]), Instruction::Error));
        assert_node!(tree, *terminator, Terminator::Return { .. });
    });
}

/// Reject conflicting reference qualifiers through parser diagnostics.
#[test]
fn test_parse_rejects_invalid_reference_qualifiers() {
    let cases = [
        (
            "type Bad = ref<int32, managed, readonly, exclusive, local>;",
            "invalid duplicate reference access",
        ),
        (
            "type Bad = ref<int32, borrowed, '_, mutable, readonly, local>;",
            "invalid duplicate reference access",
        ),
    ];

    for (source, expected) in cases {
        let (_, diagnostics) = TestParser::new(source).parse_with_diagnostics();
        let messages = diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect::<Vec<_>>();
        assert_eq!(messages, [expected], "{source}");
    }
}
