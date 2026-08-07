use std::hash::Hash;

use destack_core::FxIndexMap;
use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::Span;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Require overload signatures for one declaration to be adjacent.
    pub ADJACENT_OVERLOAD_SIGNATURES {
        id: "adjacent-overload-signatures",
        summary: "Require overload signatures for one declaration to be adjacent",
        explanation: r#"
Separated overload signatures hide the complete declaration and make its order difficult to inspect.
Keep every overload for one function or member in one uninterrupted group.
"#,
        example: {
            reported: r#"
declare function parse(value: string): string;
declare function format(value: string): string;
declare function parse(value: int32): string;
"#,
            accepted: r#"
declare function parse(value: string): string;
declare function parse(value: int32): string;
declare function format(value: string): string;
"#,
        },
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report overload declarations separated by another authored item.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect top-level function declarations in authored order
    let overloads = module
        .roots
        .iter()
        .copied()
        .map(|expression| function_overload(module, &view, expression));
    report_separated_overloads(lint, overloads, &mut output)?;

    // inspect function declarations within each authored block
    for (_, block) in view.iter_nodes::<dir::Block>() {
        let overloads = block
            .iter_expressions()
            .map(|expression| function_overload(module, &view, expression));
        report_separated_overloads(lint, overloads, &mut output)?;
    }

    // inspect declaration member lists in their authored order
    for (_, declaration) in view.iter_nodes::<dir::Declaration>() {
        // inspect nested module and global declarations
        match declaration {
            dir::Declaration::Global(declaration) => {
                let overloads = declaration
                    .expressions
                    .iter()
                    .copied()
                    .map(|expression| function_overload(module, &view, expression));
                report_separated_overloads(lint, overloads, &mut output)?;
            }
            dir::Declaration::Module(declaration) => {
                let overloads = declaration
                    .expressions
                    .iter()
                    .copied()
                    .map(|expression| function_overload(module, &view, expression));
                report_separated_overloads(lint, overloads, &mut output)?;
            }
            _ => {}
        }

        // inspect declaration members
        if let Some(members) = declaration.member_ids() {
            let overloads = members
                .iter()
                .copied()
                .map(|member| member_overload(module, &view, member));
            report_separated_overloads(lint, overloads, &mut output)?;
        }

        // inspect structural type members
        if let Some(members) = declaration.type_member_ids() {
            let overloads = members
                .iter()
                .copied()
                .map(|member| type_member_overload(module, &view, member));
            report_separated_overloads(lint, overloads, &mut output)?;
        }
    }

    Ok(output)
}

/// Report repeated callable keys that do not occupy adjacent items.
fn report_separated_overloads<K>(
    lint: &Lint,
    overloads: impl IntoIterator<Item = Result<Option<(K, Span)>, ProviderError>>,
    output: &mut LintOutput,
) -> Result<(), ProviderError>
where
    K: Eq + Hash,
{
    let mut previous = FxIndexMap::default();

    // compare each callable with its preceding declaration of the same identity
    for (index, overload) in overloads.into_iter().enumerate() {
        let Some((key, span)) = overload? else {
            continue;
        };
        if previous
            .insert(key, index)
            .is_some_and(|previous| previous + 1 != index)
        {
            output.report(lint.diagnostic("overload signature is separated from its group", span));
        }
    }

    Ok(())
}

/// Return the overload identity of one authored expression.
fn function_overload(
    module: &DirModule<'_>,
    view: &dir::View<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<(dir::StaticKey, Span)>, ProviderError> {
    // select a named function declaration
    let dir::Expression::Declaration(declaration) = view.get(expression) else {
        return Ok(None);
    };
    let dir::Declaration::Function(function) = view.get(*declaration) else {
        return Ok(None);
    };
    let Some(name) = function.name else {
        return Ok(None);
    };

    let span = module.main_span(declaration.into_any())?;

    Ok(Some((name.static_key(), span)))
}

/// Return the overload identity of one declaration member.
fn member_overload(
    module: &DirModule<'_>,
    view: &dir::View<'_>,
    member: dir::LocalNodeId<dir::Member>,
) -> Result<Option<((dir::MemberSpace, dir::MemberSlot), Span)>, ProviderError> {
    // select a callable non-accessor member
    let value = view.get(member);
    let Some(signature) = value.signature() else {
        return Ok(None);
    };
    if matches!(
        signature.role,
        Some(dir::FunctionRole::Getter | dir::FunctionRole::Setter)
    ) {
        return Ok(None);
    }
    let (Some(space), Some(slot)) = (value.space(), value.slot()) else {
        return Ok(None);
    };

    let span = module.main_span(member.into_any())?;

    Ok(Some(((space, slot), span)))
}

/// Return the overload identity of one type member.
fn type_member_overload(
    module: &DirModule<'_>,
    view: &dir::View<'_>,
    member: dir::LocalNodeId<dir::TypeMember>,
) -> Result<Option<((dir::MemberSpace, dir::MemberSlot), Span)>, ProviderError> {
    // exclude accessors from overload grouping
    let value = view.get(member);
    if value.signature().is_some_and(|signature| {
        matches!(
            signature.role,
            Some(dir::FunctionRole::Getter | dir::FunctionRole::Setter)
        )
    }) {
        return Ok(None);
    }
    let (Some(space), Some(slot)) = (value.space(), value.slot()) else {
        return Ok(None);
    };

    // select the source span for the callable form
    let span = match value {
        dir::TypeMember::Method { .. } => module.main_span(member.into_any())?,
        dir::TypeMember::CallSignature { .. } | dir::TypeMember::ConstructSignature { .. } => {
            module.span(member.into_any())?
        }
        _ => return Ok(None),
    };

    Ok(Some(((space, slot), span)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report separated method overloads.
    #[test]
    fn test_reports_separated_method_overloads() {
        let session = TestSession::dir(
            &ADJACENT_OVERLOAD_SIGNATURES,
            r#"
interface Parser {
    parse(value: string): string;
    format(value: string): string;
    parse(value: int32): string;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[adjacent-overload-signatures]: overload signature is separated from its group
 ──▶ main.ds:4:5
  │
2 │     parse(value: string): string;
3 │     format(value: string): string;
4 │     parse(value: int32): string;
  │     ^^^^^
5 │ }
  │
"#,
        );
    }

    /// Accept adjacent method overloads.
    #[test]
    fn test_accepts_adjacent_method_overloads() {
        let session = TestSession::dir(
            &ADJACENT_OVERLOAD_SIGNATURES,
            r#"
interface Parser {
    parse(value: string): string;
    parse(value: int32): string;
    format(value: string): string;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Keep getter and setter grouping under its dedicated rule.
    #[test]
    fn test_accepts_separated_accessor_pair() {
        let session = TestSession::dir(
            &ADJACENT_OVERLOAD_SIGNATURES,
            r#"
interface Store {
    get value(): string;
    clear(): void;
    set value(next: string);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report separated call signatures.
    #[test]
    fn test_reports_separated_call_signatures() {
        let session = TestSession::dir(
            &ADJACENT_OVERLOAD_SIGNATURES,
            r#"
interface Parser {
    (value: string): string;
    reset(): void;
    (value: int32): string;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[adjacent-overload-signatures]: overload signature is separated from its group
 ──▶ main.ds:4:5
  │
2 │     (value: string): string;
3 │     reset(): void;
4 │     (value: int32): string;
  │     ^^^^^^^^^^^^^^^^^^^^^^
5 │ }
  │
"#,
        );
    }

    /// Report separated construct signatures.
    #[test]
    fn test_reports_separated_construct_signatures() {
        let session = TestSession::dir(
            &ADJACENT_OVERLOAD_SIGNATURES,
            r#"
interface Factory {
    new (value: string): string;
    reset(): void;
    new (value: int32): string;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[adjacent-overload-signatures]: overload signature is separated from its group
 ──▶ main.ds:4:5
  │
2 │     new (value: string): string;
3 │     reset(): void;
4 │     new (value: int32): string;
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^
5 │ }
  │
"#,
        );
    }

    /// Keep static and instance declarations in distinct overload groups.
    #[test]
    fn test_accepts_separated_static_and_instance_methods() {
        let session = TestSession::dir(
            &ADJACENT_OVERLOAD_SIGNATURES,
            r#"
declare class Parser {
    static parse(value: string): string;
    format(value: string): string;
    parse(value: int32): string;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report separated overloads inside an anonymous module declaration.
    #[test]
    fn test_reports_separated_module_overloads() {
        let session = TestSession::dir(
            &ADJACENT_OVERLOAD_SIGNATURES,
            r#"
module {
    declare function parse(value: string): string;
    declare function format(value: string): string;
    declare function parse(value: int32): string;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[adjacent-overload-signatures]: overload signature is separated from its group
 ──▶ main.ds:4:22
  │
2 │     declare function parse(value: string): string;
3 │     declare function format(value: string): string;
4 │     declare function parse(value: int32): string;
  │                      ^^^^^
5 │ }
  │
"#,
        );
    }

    /// Report separated overloads inside an ambient global declaration.
    #[test]
    fn test_reports_separated_global_overloads() {
        let session = TestSession::dir(
            &ADJACENT_OVERLOAD_SIGNATURES,
            r#"
declare global {
    declare function parse(value: string): string;
    declare function format(value: string): string;
    declare function parse(value: int32): string;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[adjacent-overload-signatures]: overload signature is separated from its group
 ──▶ main.ds:4:22
  │
2 │     declare function parse(value: string): string;
3 │     declare function format(value: string): string;
4 │     declare function parse(value: int32): string;
  │                      ^^^^^
5 │ }
  │
"#,
        );
    }
}
