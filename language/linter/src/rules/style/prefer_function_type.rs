use destack_core::FxIndexSet;
use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::Patch;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer direct callable types over single-signature structural types.
    pub PREFER_FUNCTION_TYPE {
        id: "prefer-function-type",
        summary: "Prefer function types over single-signature structural types",
        explanation: r#"
A structural type containing only one callable signature adds structure without expressing another capability.
Instead, you SHOULD write the callable directly as a function or constructor type.

Nominal, inherited, and overloaded callable types retain their declarations.
"#,
        example: {
            reported: r#"
type Transform = { (value: int32): string };
"#,
            accepted: r#"
type Transform = (value: int32) => string;
"#,
        },
        provenance: [TypeScriptEslint("prefer-function-type")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report structural types whose only capability is one callable signature.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut this_type_ancestors = FxIndexSet::default();
    let mut output = LintOutput::default();

    // index every subtree whose meaning depends on an enclosing `this` type
    for (node, ty) in view.iter_nodes::<dir::TypeExpression>() {
        if !matches!(ty, dir::TypeExpression::This) {
            continue;
        }
        let mut current = Some(node.into_any());
        while let Some(node) = current {
            this_type_ancestors.insert(node);
            current = view.get_parent_any(node);
        }
    }

    // inspect object type expressions containing exactly one callable signature
    for (expression, ty) in view.iter_nodes::<dir::TypeExpression>() {
        let dir::TypeExpression::Object { members } = ty else {
            continue;
        };
        let [member] = members.as_slice() else {
            continue;
        };
        let return_type = match view.get(*member) {
            dir::TypeMember::CallSignature { signature } => signature.return_type,
            dir::TypeMember::ConstructSignature { signature } => signature.return_type,
            _ => continue,
        };
        let Some(return_type) = return_type else {
            continue;
        };

        // rewrite the object type when no authored comments would be discarded
        let extent = module.source_extent(expression.into_any())?;
        let member_extent = module.source_extent(member.into_any())?;
        let return_extent = module.source_extent(return_type.into_any())?;
        let mut diagnostic =
            lint.diagnostic("object type contains only one callable signature", extent);
        if !this_type_ancestors.contains(&member.into_any())
            && let Some(replacement) = callable_type_source(
                extent,
                member_extent,
                return_extent,
                function_type_needs_parentheses(expression, return_type, module),
                module,
            )?
        {
            let suggestion =
                lint.fix("write a function type", Patch::replace(extent, replacement))?;
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    // inspect structural interfaces containing exactly one callable signature
    for (declaration, value) in view.iter_nodes::<dir::Declaration>() {
        let dir::Declaration::Interface(interface) = value else {
            continue;
        };
        if interface.is_nominal || !interface.extends_types.is_empty() {
            continue;
        }
        let [member] = interface.members.as_slice() else {
            continue;
        };
        let return_type = match view.get(*member) {
            dir::TypeMember::CallSignature { signature } => signature.return_type,
            dir::TypeMember::ConstructSignature { signature } => signature.return_type,
            _ => continue,
        };
        let Some(return_type) = return_type else {
            continue;
        };

        let span = module.main_span(declaration.into_any())?;
        let mut diagnostic =
            lint.diagnostic("interface contains only one callable signature", span);
        if !this_type_ancestors.contains(&member.into_any())
            && let Some(replacement) = callable_interface_source(
                declaration,
                member.into_any(),
                return_type.into_any(),
                module,
            )?
        {
            let extent = module.source_extent(declaration.into_any())?;
            let suggestion = lint.fix(
                "write a function type alias",
                Patch::replace(extent, replacement),
            )?;
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Rewrite one callable member as a function or constructor type.
fn callable_type_source(
    object: destack_source::Span,
    member: destack_source::Span,
    return_type: destack_source::Span,
    needs_parentheses: bool,
    module: &DirModule<'_>,
) -> Result<Option<String>, ProviderError> {
    if module.has_unretained_comment(object, &[])? {
        return Ok(None);
    }

    // require one coherent authored span chain
    if object.file != member.file
        || member.file != return_type.file
        || !object.contains_span(member)
        || !member.contains_span(return_type)
    {
        return Err(ProviderError::internal(
            "callable signature spans do not share one containing object type",
        ));
    }

    // locate the return separator immediately before the return type
    let source = module.file(member.file)?.text();
    let mut separator = return_type.start as usize;
    while separator > member.start as usize
        && source.as_bytes()[separator - 1].is_ascii_whitespace()
    {
        separator -= 1;
    }
    if separator == member.start as usize || source.as_bytes()[separator - 1] != b':' {
        return Err(ProviderError::internal(
            "callable signature return type has no preceding colon",
        ));
    }
    separator -= 1;

    // preserve the authored parameters and return type around the arrow
    let parameters = &source[member.start as usize..separator];
    let return_type = module.source(return_type)?;
    let replacement = format!("{} => {return_type}", parameters.trim_end());
    let replacement = if needs_parentheses {
        format!("({replacement})")
    } else {
        replacement
    };

    Ok(Some(replacement))
}

/// Rewrite one structural callable interface as a type alias.
fn callable_interface_source(
    declaration: dir::LocalNodeId<dir::Declaration>,
    member: dir::LocalNodeIdAny,
    return_type: dir::LocalNodeIdAny,
    module: &DirModule<'_>,
) -> Result<Option<String>, ProviderError> {
    let extent = module.source_extent(declaration.into_any())?;
    if module.has_unretained_comment(extent, &[])? {
        return Ok(None);
    }

    // split the declaration head from its sole member
    let member = module.source_extent(member)?;
    let return_type = module.source_extent(return_type)?;
    let source = module.file(extent.file)?.text();
    let head = source
        .get(extent.start as usize..member.start as usize)
        .ok_or_else(|| ProviderError::internal("callable interface spans are not nested"))?;
    let body_start = head.rfind('{').ok_or_else(|| {
        ProviderError::internal("callable interface has no opening body delimiter")
    })?;
    let mut head = head[..body_start].trim_end().to_string();
    let keyword = head
        .rfind("interface")
        .ok_or_else(|| ProviderError::internal("callable interface has no interface keyword"))?;
    head.replace_range(keyword..keyword + "interface".len(), "type");

    // preserve the callable member after changing its return separator
    let callable = callable_type_source(member, member, return_type, false, module)?
        .ok_or_else(|| ProviderError::internal("comment-free callable member was not rewritten"))?;

    // terminate the replacement unless the authored declaration already does
    let has_terminator = source
        .get(extent.end as usize..)
        .is_some_and(|suffix| suffix.starts_with(';'));
    let terminator = if has_terminator { "" } else { ";" };

    Ok(Some(format!("{head} = {callable}{terminator}")))
}

/// Return whether one callable replacement needs grouping in its parent.
fn function_type_needs_parentheses(
    mut expression: dir::LocalNodeId<dir::TypeExpression>,
    return_type: dir::LocalNodeId<dir::TypeExpression>,
    module: &DirModule<'_>,
) -> bool {
    let view = module.view();
    let mut child = expression;

    // skip transparent one-element unions and intersections
    let parent = loop {
        let Some(parent) = view.get_parent_for(expression) else {
            return false;
        };
        if parent.ty != dir::NodeType::TypeExpression {
            break parent;
        }
        let parent_type = dir::LocalNodeId::<dir::TypeExpression>::new(parent.id);
        let is_transparent = matches!(
            view.get(parent_type),
            dir::TypeExpression::Union { elements }
                | dir::TypeExpression::Intersection { elements }
                if elements.as_slice() == [expression]
        );
        if !is_transparent {
            break parent;
        }

        child = parent_type;
        expression = parent_type;
    };

    // group callable types in lower-precedence type positions
    if parent.ty == dir::NodeType::TypeExpression {
        let parent = dir::LocalNodeId::<dir::TypeExpression>::new(parent.id);

        return match view.get(parent) {
            dir::TypeExpression::Array { element } => *element == child,
            dir::TypeExpression::Index { left, .. } => *left == child,
            dir::TypeExpression::Readonly { target_type }
            | dir::TypeExpression::KeyOf { target_type }
            | dir::TypeExpression::Must { target_type }
            | dir::TypeExpression::Not { target_type }
            | dir::TypeExpression::OwnedOf { target_type, .. }
            | dir::TypeExpression::BorrowedOf { target_type, .. }
            | dir::TypeExpression::PointerOf { target_type, .. } => *target_type == child,
            dir::TypeExpression::Conditional {
                left, extends_type, ..
            } => {
                *left == child
                    || *extends_type == child
                        && matches!(
                            view.get(return_type),
                            dir::TypeExpression::Infer {
                                constraint: Some(_),
                                ..
                            }
                        )
            }
            dir::TypeExpression::Extends { left, right }
            | dir::TypeExpression::Implements { left, right } => *left == child || *right == child,
            dir::TypeExpression::Union { elements }
            | dir::TypeExpression::Intersection { elements } => elements.len() > 1,
            dir::TypeExpression::Range { .. } => true,
            _ => false,
        };
    }

    // group function return annotations on lambda declarations
    if parent.ty == dir::NodeType::Declaration {
        let parent = dir::LocalNodeId::<dir::Declaration>::new(parent.id);
        let dir::Declaration::Function(function) = view.get(parent) else {
            return false;
        };

        return function.signature.form == dir::FunctionForm::Lambda
            && function.signature.return_type == Some(child);
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace a sole generic call signature with a function type.
    #[test]
    fn test_replaces_generic_call_signature() {
        let session = TestSession::dir(
            &PREFER_FUNCTION_TYPE,
            r#"
type Identity = { <T>(value: T): T };
"#,
        );

        session.assert_fixes(
            r#"
type Identity = <T>(value: T) => T;
"#,
        );
    }

    /// Preserve callable precedence when replacing a union arm.
    #[test]
    fn test_groups_replaced_union_call_signature() {
        let session = TestSession::dir(
            &PREFER_FUNCTION_TYPE,
            r#"
type OptionalCallback = { (): string } | undefined;
"#,
        );

        session.assert_fixes(
            r#"
type OptionalCallback = (() => string) | undefined;
"#,
        );
    }

    /// Preserve callable precedence when replacing an array element.
    #[test]
    fn test_groups_replaced_array_call_signature() {
        let session = TestSession::dir(
            &PREFER_FUNCTION_TYPE,
            r#"
type Callbacks = { (): void }[];
"#,
        );

        session.assert_fixes(
            r#"
type Callbacks = (() => void)[];
"#,
        );
    }

    /// Replace a sole construct signature with a constructor type.
    #[test]
    fn test_replaces_construct_signature() {
        let session = TestSession::dir(
            &PREFER_FUNCTION_TYPE,
            r#"
declare class User {}

type Factory = { new (name: string): User };
"#,
        );

        session.assert_fixes(
            r#"
declare class User {}

type Factory = new (name: string) => User;
"#,
        );
    }

    /// Replace a structural callable interface with a type alias.
    #[test]
    fn test_replaces_structural_callable_interface() {
        let session = TestSession::dir(
            &PREFER_FUNCTION_TYPE,
            r#"
interface Predicate {
    (value: int32): boolean;
}
"#,
        );

        session.assert_fixes(
            r#"
type Predicate = (value: int32) => boolean;
"#,
        );
    }

    /// Report a `this`-relative callable without changing its meaning.
    #[test]
    fn test_reports_this_relative_callable_without_fix() {
        let session = TestSession::dir(
            &PREFER_FUNCTION_TYPE,
            r#"
type Fluent = { (): this };
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-function-type]: object type contains only one callable signature
 ──▶ main.ds:1:15
  │
1 │ type Fluent = { (): this };
  │               ^^^^^^^^^^^^
  │
"#,
        );
    }

    /// Accept a nominal callable interface.
    #[test]
    fn test_accepts_nominal_callable_interface() {
        let session = TestSession::dir(
            &PREFER_FUNCTION_TYPE,
            r#"
newtype interface Predicate {
    (value: int32): boolean;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept structural callable types with overloads or inherited capabilities.
    #[test]
    fn test_accepts_overloaded_and_extended_interfaces() {
        let session = TestSession::dir(
            &PREFER_FUNCTION_TYPE,
            r#"
interface Named {
    name: string;
}

interface Overloaded {
    (value: int32): string;
    (value: string): int32;
}

interface NamedFunction extends Named {
    (): void;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
