use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer optional chaining to equivalent nullish conditionals.
    pub PREFER_OPTIONAL_CHAIN {
        id: "prefer-optional-chain",
        summary: "Prefer optional chaining to equivalent nullish conditionals",
        explanation: r#"
A conditional that returns undefined for a nullish receiver manually implements optional access.
Instead, you SHOULD use optional chaining at the guarded access.

The guarded receiver must produce the same value without observable effects each time it appears.
"#,
        example: {
            reported: r#"
interface User {
    name: string;
}

function name(user: User | undefined): string | undefined {
    return user === undefined ? undefined : user.name;
}
"#,
            accepted: r#"
interface User {
    name: string;
}

function name(user: User | undefined): string | undefined {
    return user?.name;
}
"#,
        },
        provenance: [TypeScriptEslint("prefer-optional-chain")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report conditionals equivalent to one optional access chain.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect conditionals that test one receiver for nullish values
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::If {
            condition,
            then_expression,
            else_expression: Some(else_expression),
            ..
        } = node
        else {
            continue;
        };
        let Some(condition) = condition.as_expression() else {
            continue;
        };
        let Some(then_expression) = module.sole_value_expression(*then_expression) else {
            continue;
        };
        let Some(else_expression) = module.sole_value_expression(*else_expression) else {
            continue;
        };
        let Some(test) = module.nullish_test(condition)? else {
            continue;
        };
        let (selected, nullish) = if test.is_defined {
            (then_expression, else_expression)
        } else {
            (else_expression, then_expression)
        };
        if view.get(nullish).as_scalar() != Some(dir::Literal::Undefined)
            || !module.is_duplicable_expression(test.value)?
        {
            continue;
        }
        let Some((position, marker)) = optional_position(module, selected, test.value)? else {
            continue;
        };

        // replace the complete conditional while retaining the selected access text
        let extent = module.source_extent(expression.into_any())?;
        let selected_extent = module.source_extent(selected.into_any())?;
        let mut diagnostic =
            lint.diagnostic("conditional manually guards an optional access", extent);
        if !module.has_unretained_comment(extent, &[selected_extent])? {
            let fix = fix(module, lint, extent, selected_extent, position, marker)?;
            diagnostic = diagnostic.suggestion(fix);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Return the insertion position and marker for one guarded postfix access.
fn optional_position(
    module: &DirModule<'_>,
    selected: dir::LocalNodeId<dir::Expression>,
    guarded: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<(u32, &'static str)>, ProviderError> {
    let view = module.view();
    let mut current = selected;

    // descend through the selected postfix chain to the guarded receiver
    loop {
        let (receiver, is_optional, marker) = match view.get(current) {
            dir::Expression::Member {
                left, is_optional, ..
            } => (*left, *is_optional, "?"),
            dir::Expression::Index {
                left, is_optional, ..
            }
            | dir::Expression::Call {
                left, is_optional, ..
            } => (*left, *is_optional, "?."),
            dir::Expression::Instantiation { left, .. } => {
                current = *left;
                continue;
            }
            _ => return Ok(None),
        };
        if module.is_same_computation(receiver, guarded)? {
            if is_optional {
                return Ok(None);
            }

            let position = module.source_extent(receiver.into_any())?.end;

            return Ok(Some((position, marker)));
        }

        current = receiver;
    }
}

/// Insert one optional marker into the selected access expression.
fn fix(
    module: &DirModule<'_>,
    lint: &Lint,
    target: tspp_source::Span,
    selected: tspp_source::Span,
    position: u32,
    marker: &str,
) -> Result<DiagnosticSuggestion, ProviderError> {
    if position < selected.start || position > selected.end {
        return Err(ProviderError::internal(format!(
            "optional marker position {position} lies outside selected access {selected:?}"
        )));
    }

    // insert the marker into the exact retained source
    let mut source = module.source(selected)?.to_string();
    let offset = (position - selected.start) as usize;
    source.insert_str(offset, marker);
    let patch = Patch::replace(target, source);
    let fix = lint.fix("use optional chaining", patch)?;

    Ok(fix)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace a guarded member access.
    #[test]
    fn test_replaces_member_access() {
        let session = TestSession::dir(
            &PREFER_OPTIONAL_CHAIN,
            r#"
interface User {
    name: string;
}

function name(user: User | undefined): string | undefined {
    return user === undefined ? undefined : user.name;
}
"#,
        );

        session.assert_fixes(
            r#"
interface User {
    name: string;
}

function name(user: User | undefined): string | undefined {
    return user?.name;
}
"#,
        );
    }

    /// Replace a guarded access returned from a value-producing if expression.
    #[test]
    fn test_replaces_if_expression() {
        let session = TestSession::dir(
            &PREFER_OPTIONAL_CHAIN,
            r#"
interface User {
    name: string;
}

function name(user: User | undefined): string | undefined {
    return if (user === undefined) {
        undefined
    } else {
        user.name
    };
}
"#,
        );

        session.assert_fixes(
            r#"
interface User {
    name: string;
}

function name(user: User | undefined): string | undefined {
    return user?.name;
}
"#,
        );
    }

    /// Replace a guarded nested member chain at its first access.
    #[test]
    fn test_replaces_nested_member_access() {
        let session = TestSession::dir(
            &PREFER_OPTIONAL_CHAIN,
            r#"
interface Profile {
    name: string;
}
interface User {
    profile: Profile;
}

function name(user: User | undefined): string | undefined {
    return user === undefined ? undefined : user.profile.name;
}
"#,
        );

        session.assert_fixes(
            r#"
interface Profile {
    name: string;
}
interface User {
    profile: Profile;
}

function name(user: User | undefined): string | undefined {
    return user?.profile.name;
}
"#,
        );
    }

    /// Replace a guarded optional call.
    #[test]
    fn test_replaces_function_call() {
        let session = TestSession::dir(
            &PREFER_OPTIONAL_CHAIN,
            r#"
function invoke(callback: (() => int32) | undefined): int32 | undefined {
    return callback === undefined ? undefined : callback();
}
"#,
        );

        session.assert_fixes(
            r#"
function invoke(callback: (() => int32) | undefined): int32 | undefined {
    return callback?.();
}
"#,
        );
    }

    /// Replace an indexed access guarded by an undefined test.
    #[test]
    fn test_replaces_index_access() {
        let session = TestSession::dir(
            &PREFER_OPTIONAL_CHAIN,
            r#"
function first(values: string[] | undefined): string | undefined {
    return values === undefined ? undefined : values[0];
}
"#,
        );

        session.assert_fixes(
            r#"
function first(values: string[] | undefined): string | undefined {
    return values?.[0];
}
"#,
        );
    }

    /// Accept a nullish branch whose value optional chaining would change.
    #[test]
    fn test_accepts_null_result_branch() {
        let session = TestSession::dir(
            &PREFER_OPTIONAL_CHAIN,
            r#"
interface User {
    name: string;
}

function name(user: User | null): string | null {
    return user === null ? null : user.name;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept repeated receivers whose evaluations may differ.
    #[test]
    fn test_accepts_effectful_receiver() {
        let session = TestSession::dir(
            &PREFER_OPTIONAL_CHAIN,
            r#"
interface User {
    name: string;
}
declare function next(): User | undefined;

function name(): string | undefined {
    return next() === undefined ? undefined : next()!.name;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
