use tspp_core::FxIndexSet;
use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::Span;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Require documentation for named declarations.
    pub MISSING_DOCS {
        id: "missing-docs",
        summary: "Require documentation for named declarations",
        explanation: r#"
Undocumented declarations hide their purpose and constraints from readers and generated references.
Instead, you SHOULD document every type, function, constant, nominal member, variant, and direct
member of a named object type.

Exact interface implementations inherit the documentation of their requirement.
Anonymous structural types and index signatures inherit the documentation of their enclosing
declaration.
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
        provenance: [Rustc("missing_docs")],
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
        .members
        .member_conformances()
        .filter(|conformance| conformance.member != conformance.requirement)
        .map(|conformance| conformance.member)
        .collect::<FxIndexSet<_>>();
    let mut missing = Vec::new();
    let mut output = LintOutput::default();

    // inspect every declaration node once
    for node in view.iter_node_ids() {
        let Some((noun, documented, target)) = required_documentation(node, &view, module)? else {
            continue;
        };
        if view.get_documentation_any(documented).is_some() {
            continue;
        }

        // inherit documentation for exact interface implementations
        if matches!(node.ty, dir::NodeType::Member) {
            let global = node.into_global(module.id);
            let symbol = module.bindings.declaration_symbol(global).ok_or_else(|| {
                ProviderError::internal(format!("declaration member {global:?} has no symbol"))
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
            dir::TypeMember::CallSignature { .. } | dir::TypeMember::ConstructSignature { .. }
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
    module: &DirModule<'_>,
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
            type_member_documentation(member, view)?
        }
        dir::NodeType::Declarator => {
            let declarator = dir::LocalNodeId::new(node.id);

            return constant_documentation(declarator, view, module);
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
) -> Result<Option<&'static str>, ProviderError> {
    // let an index signature inherit the named declaration's documentation
    if matches!(view.get(member), dir::TypeMember::IndexSignature { .. }) {
        return Ok(None);
    }
    if !is_named_object_member(member, view)? {
        return Ok(None);
    }

    // name each documented structural member kind
    let noun = match view.get(member) {
        dir::TypeMember::Field { .. } => "field",
        dir::TypeMember::Method { signature, .. } => match signature.role {
            Some(dir::FunctionRole::Getter) => "getter",
            Some(dir::FunctionRole::Setter) => "setter",
            _ => "method",
        },
        dir::TypeMember::CallSignature { .. } => "call signature",
        dir::TypeMember::ConstructSignature { .. } => "construct signature",
        dir::TypeMember::AssociatedType { .. } => "associated type",
        dir::TypeMember::AssociatedConst { .. } => "associated constant",
        dir::TypeMember::IndexSignature { .. } | dir::TypeMember::Error => return Ok(None),
    };

    Ok(Some(noun))
}

/// Return whether a type member belongs directly to a named object type.
fn is_named_object_member(
    member: dir::LocalNodeId<dir::TypeMember>,
    view: &dir::View<'_>,
) -> Result<bool, ProviderError> {
    let mut child = member.into_any();

    // follow object and intersection types to their declaration
    loop {
        let parent = view.get_parent_any(child).ok_or_else(|| {
            ProviderError::internal(format!("type member {member:?} has no declaration parent"))
        })?;
        match parent.ty {
            dir::NodeType::Declaration => {
                let declaration = dir::LocalNodeId::new(parent.id);
                let is_named = match view.get(declaration) {
                    dir::Declaration::Type(_) => true,
                    dir::Declaration::Interface(declaration) => declaration.name.is_some(),
                    _ => false,
                };

                return Ok(is_named);
            }
            dir::NodeType::TypeExpression => {
                let expression = dir::LocalNodeId::new(parent.id);
                if !matches!(
                    view.get(expression),
                    dir::TypeExpression::Object { .. } | dir::TypeExpression::Intersection { .. }
                ) {
                    return Ok(false);
                }
            }
            _ => return Ok(false),
        }

        child = parent;
    }
}

/// Return required documentation for one module constant.
fn constant_documentation(
    declarator: dir::LocalNodeId<dir::Declarator>,
    view: &dir::View<'_>,
    module: &DirModule<'_>,
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
    if !module.is_module_expression(expression)? {
        return Ok(None);
    }

    Ok(Some((
        "constant",
        expression.into_any(),
        view.get(declarator).pattern.into_any(),
    )))
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
 ──▶ main.tspp:1:8
  │
1 │ struct Session {
  │        ^^^^^^^
2 │     userId: string;
3 │
  │

warning[missing-docs]: field must have documentation
 ──▶ main.tspp:2:5
  │
1 │ struct Session {
2 │     userId: string;
  │     ^^^^^^
3 │
4 │     reset(): void {}
  │

warning[missing-docs]: method must have documentation
 ──▶ main.tspp:4:5
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
 ──▶ main.tspp:1:7
  │
1 │ const LIMIT = 1;
  │       ^^^^^
2 │
3 │ enum Status {
  │

warning[missing-docs]: enum must have documentation
 ──▶ main.tspp:3:6
  │
1 │ const LIMIT = 1;
2 │
3 │ enum Status {
  │      ^^^^^^
4 │     Pending,
5 │ }
  │

warning[missing-docs]: variant must have documentation
 ──▶ main.tspp:4:5
  │
2 │
3 │ enum Status {
4 │     Pending,
  │     ^^^^^^^
5 │ }
6 │
  │

warning[missing-docs]: interface must have documentation
 ──▶ main.tspp:7:11
  │
5 │ }
6 │
7 │ interface Factory {
  │           ^^^^^^^
8 │     (): Status;
9 │ }
  │

warning[missing-docs]: call signature must have documentation
 ──▶ main.tspp:8:5
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

    /// Accept members of anonymous structural types and index signatures.
    #[test]
    fn test_accepts_anonymous_structural_members() {
        let session = TestSession::dir(
            &MISSING_DOCS,
            r#"
/// One operation outcome.
type Outcome<T> =
    | {
        kind: "ok";

        value: T;
    }
    | {
        kind: "error";

        message: string;
    };

/// Named labels.
type Labels = {
    readonly [key: string]: string;
};

/// Process one operation.
declare function process(options: {
    mode: string;

    nested: {
        enabled: boolean;
    };
}): Outcome<string>;
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report members contributed to a named object type through an intersection.
    #[test]
    fn test_reports_undocumented_named_object_member() {
        let session = TestSession::dir(
            &MISSING_DOCS,
            r#"
/// Base request options.
type BaseOptions = {};

/// Request options.
type RequestOptions = BaseOptions & {
    retries: int32;
};
"#,
        );

        session.assert_diagnostics(
            r#"
warning[missing-docs]: field must have documentation
 ──▶ main.tspp:6:5
  │
4 │ /// Request options.
5 │ type RequestOptions = BaseOptions & {
6 │     retries: int32;
  │     ^^^^^^^
7 │ };
  │
"#,
        );
    }
}
