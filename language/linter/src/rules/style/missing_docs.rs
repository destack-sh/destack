use destack_core::FxIndexSet;
use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::Span;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Require documentation for named declarations.
    pub MISSING_DOCS {
        id: "missing-docs",
        summary: "Require documentation for named declarations",
        explanation: r#"
Undocumented declarations hide their purpose and constraints from readers and generated references.
Instead, you SHOULD document every type, function, constant, member, field, and variant.

Exact interface implementations inherit the documentation of their requirement.
"#,
        example: {
            reported: r#"
struct Session {
    /// The authenticated user.
    userId: string;
}
"#,
            accepted: r#"
/// One authenticated user session.
struct Session {
    /// The authenticated user.
    userId: string;
}
"#,
        },
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report declarations without attached documentation.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let implementations = module
        .definitions
        .member_conformances()
        .filter(|conformance| conformance.member != conformance.requirement)
        .map(|conformance| conformance.member)
        .collect::<FxIndexSet<_>>();
    let mut missing = Vec::new();
    let mut output = LintOutput::default();

    // inspect every declaration node once
    for node in view.iter_node_ids() {
        let Some((noun, documented, target)) = required_documentation(node, &view, module.roots)?
        else {
            continue;
        };
        if view.get_documentation_any(documented).is_some() {
            continue;
        }

        // inherit documentation for exact interface implementations
        if matches!(node.ty, dir::NodeType::Member) {
            let global = node.into_global(module.id);
            let symbol = module.bindings.declaration_symbol(global).ok_or_else(|| {
                ProviderError::internal(format!(
                    "checked declaration member {global:?} has no symbol"
                ))
            })?;
            if implementations.contains(&symbol.into_global(module.id)) {
                continue;
            }
        }

        // retain the declaration name in physical source order
        let span = documentation_span(target, &view, module)?;
        missing.push((span, noun));
    }

    // report declarations in physical source order
    missing.sort_unstable_by_key(|(span, _)| (span.file, span.start, span.end));
    for (span, noun) in missing {
        output.report(lint.diagnostic(format!("{noun} must have documentation"), span));
    }

    Ok(output)
}

/// Return the diagnostic span for one documented declaration shape.
fn documentation_span(
    node: dir::LocalNodeIdAny,
    view: &dir::View<'_>,
    module: &DirModule<'_>,
) -> Result<Span, ProviderError> {
    let uses_extent = match node.ty {
        dir::NodeType::TypeMember => matches!(
            view.get(dir::LocalNodeId::<dir::TypeMember>::new(node.id)),
            dir::TypeMember::CallSignature { .. }
                | dir::TypeMember::ConstructSignature { .. }
                | dir::TypeMember::IndexSignature { .. }
        ),
        dir::NodeType::Pattern => true,
        _ => false,
    };

    // anchor named declarations to their name and anonymous shapes to their full extent
    if uses_extent {
        module.source_extent(node)
    } else {
        module.main_span(node)
    }
}

/// Return the noun, documentation node, and diagnostic target for one declaration.
fn required_documentation(
    node: dir::LocalNodeIdAny,
    view: &dir::View<'_>,
    roots: &[dir::LocalNodeId<dir::Expression>],
) -> Result<Option<(&'static str, dir::LocalNodeIdAny, dir::LocalNodeIdAny)>, ProviderError> {
    let noun = match node.ty {
        dir::NodeType::Declaration => {
            let declaration = dir::LocalNodeId::new(node.id);
            declaration_documentation(declaration, view)
        }
        dir::NodeType::Member => {
            let member = dir::LocalNodeId::new(node.id);
            member_documentation(member, view)
        }
        dir::NodeType::TypeMember => {
            let member = dir::LocalNodeId::new(node.id);
            type_member_documentation(member, view)
        }
        dir::NodeType::Declarator => {
            let declarator = dir::LocalNodeId::new(node.id);

            return constant_documentation(declarator, view, roots);
        }
        dir::NodeType::EnumField => Some("variant"),
        _ => return Ok(None),
    };
    let Some(noun) = noun else {
        return Ok(None);
    };

    Ok(Some((noun, node, node)))
}

/// Return required documentation for one named declaration.
fn declaration_documentation(
    declaration: dir::LocalNodeId<dir::Declaration>,
    view: &dir::View<'_>,
) -> Option<&'static str> {
    let noun = match view.get(declaration) {
        dir::Declaration::Type(_) | dir::Declaration::Struct(_) => "type",
        dir::Declaration::Class(declaration) if declaration.name.is_some() => "class",
        dir::Declaration::Enum(declaration) if declaration.name.is_some() => "enum",
        dir::Declaration::Interface(declaration) if declaration.name.is_some() => "interface",
        dir::Declaration::Extension(declaration) if declaration.name.is_some() => "extension",
        dir::Declaration::Function(declaration) if declaration.name.is_some() => "function",
        _ => return None,
    };

    Some(noun)
}

/// Return required documentation for one declaration member.
fn member_documentation(
    member: dir::LocalNodeId<dir::Member>,
    view: &dir::View<'_>,
) -> Option<&'static str> {
    let noun = match view.get(member) {
        dir::Member::AssociatedType { .. } => "associated type",
        dir::Member::AssociatedConst { .. } => "associated constant",
        dir::Member::Field { .. } => "field",
        dir::Member::Method { signature, .. } => match signature.role {
            Some(dir::FunctionRole::Constructor | dir::FunctionRole::New) => "constructor",
            Some(dir::FunctionRole::Getter) => "getter",
            Some(dir::FunctionRole::Setter) => "setter",
            _ => "method",
        },
        dir::Member::StaticBlock { .. } | dir::Member::ConstBlock { .. } | dir::Member::Error => {
            return None;
        }
    };

    Some(noun)
}

/// Return required documentation for one structural type member.
fn type_member_documentation(
    member: dir::LocalNodeId<dir::TypeMember>,
    view: &dir::View<'_>,
) -> Option<&'static str> {
    let noun = match view.get(member) {
        dir::TypeMember::Field { .. } => "field",
        dir::TypeMember::Method { signature, .. } => match signature.role {
            Some(dir::FunctionRole::Getter) => "getter",
            Some(dir::FunctionRole::Setter) => "setter",
            _ => "method",
        },
        dir::TypeMember::CallSignature { .. } => "call signature",
        dir::TypeMember::ConstructSignature { .. } => "construct signature",
        dir::TypeMember::IndexSignature { .. } => "index signature",
        dir::TypeMember::AssociatedType { .. } => "associated type",
        dir::TypeMember::AssociatedConst { .. } => "associated constant",
        dir::TypeMember::Error => return None,
    };

    Some(noun)
}

/// Return required documentation for one module constant.
fn constant_documentation(
    declarator: dir::LocalNodeId<dir::Declarator>,
    view: &dir::View<'_>,
    roots: &[dir::LocalNodeId<dir::Expression>],
) -> Result<Option<(&'static str, dir::LocalNodeIdAny, dir::LocalNodeIdAny)>, ProviderError> {
    let parent = view.get_parent_for(declarator).ok_or_else(|| {
        ProviderError::internal(format!("declarator {declarator:?} has no parent"))
    })?;
    if parent.ty != dir::NodeType::Expression {
        return Err(ProviderError::internal(format!(
            "declarator {declarator:?} has non-expression parent {parent:?}"
        )));
    }
    let expression = dir::LocalNodeId::new(parent.id);
    let dir::Expression::Let {
        kind: dir::LetKind::Const,
        ..
    } = view.get(expression)
    else {
        return Ok(None);
    };
    if !is_module_expression(expression, view, roots)? {
        return Ok(None);
    }

    Ok(Some((
        "constant",
        expression.into_any(),
        view.get(declarator).pattern.into_any(),
    )))
}

/// Return whether one expression belongs directly to module or global scope.
fn is_module_expression(
    expression: dir::LocalNodeId<dir::Expression>,
    view: &dir::View<'_>,
    roots: &[dir::LocalNodeId<dir::Expression>],
) -> Result<bool, ProviderError> {
    if roots.contains(&expression) {
        return Ok(true);
    }
    let parent = view.get_parent_for(expression).ok_or_else(|| {
        ProviderError::internal(format!("expression {expression:?} has no parent"))
    })?;
    if parent.ty != dir::NodeType::Declaration {
        return Ok(false);
    };
    let parent = dir::LocalNodeId::new(parent.id);

    Ok(matches!(
        view.get(parent),
        dir::Declaration::Global(_) | dir::Declaration::Module(_)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report undocumented declarations at each nominal level.
    #[test]
    fn test_reports_undocumented_declarations() {
        let session = TestSession::dir(
            &MISSING_DOCS,
            r#"
struct Session {
    userId: string;

    reset(): void {}
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[missing-docs]: type must have documentation
 ──▶ main.ds:1:8
  │
1 │ struct Session {
  │        ^^^^^^^
2 │     userId: string;
3 │
  │

warning[missing-docs]: field must have documentation
 ──▶ main.ds:2:5
  │
1 │ struct Session {
2 │     userId: string;
  │     ^^^^^^
3 │
4 │     reset(): void {}
  │

warning[missing-docs]: method must have documentation
 ──▶ main.ds:4:5
  │
2 │     userId: string;
3 │
4 │     reset(): void {}
  │     ^^^^^
5 │ }
  │
"#,
        );
    }

    /// Accept inherited documentation on an exact interface implementation.
    #[test]
    fn test_accepts_documented_interface_implementation() {
        let session = TestSession::dir(
            &MISSING_DOCS,
            r#"
/// Resettable values.
newtype interface Resettable {
    /// Reset this value.
    reset(): void;
}

/// One session.
struct Session {}

extension of Session implements Resettable {
    reset(): void {}
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report undocumented module constants, variants, and structural signatures.
    #[test]
    fn test_reports_undocumented_module_and_type_items() {
        let session = TestSession::dir(
            &MISSING_DOCS,
            r#"
const LIMIT = 1;

enum Status {
    Pending,
}

interface Factory {
    (): Status;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[missing-docs]: constant must have documentation
 ──▶ main.ds:1:7
  │
1 │ const LIMIT = 1;
  │       ^^^^^
2 │
3 │ enum Status {
  │

warning[missing-docs]: enum must have documentation
 ──▶ main.ds:3:6
  │
1 │ const LIMIT = 1;
2 │
3 │ enum Status {
  │      ^^^^^^
4 │     Pending,
5 │ }
  │

warning[missing-docs]: variant must have documentation
 ──▶ main.ds:4:5
  │
2 │
3 │ enum Status {
4 │     Pending,
  │     ^^^^^^^
5 │ }
6 │
  │

warning[missing-docs]: interface must have documentation
 ──▶ main.ds:7:11
  │
5 │ }
6 │
7 │ interface Factory {
  │           ^^^^^^^
8 │     (): Status;
9 │ }
  │

warning[missing-docs]: call signature must have documentation
 ──▶ main.ds:8:5
  │
6 │
7 │ interface Factory {
8 │     (): Status;
  │     ^^^^^^^^^^
9 │ }
  │
"#,
        );
    }

    /// Accept documentation attached to a module constant declaration.
    #[test]
    fn test_accepts_documented_module_constant() {
        let session = TestSession::dir(
            &MISSING_DOCS,
            r#"
/// The maximum number of attempts.
const ATTEMPT_LIMIT = 3;
"#,
        );

        session.assert_no_diagnostics();
    }
}
