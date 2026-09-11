use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer passing small Copy scalars by value.
    pub NEEDLESS_BORROW_OF_COPY {
        id: "needless-borrow-of-copy",
        summary: "Prefer passing small Copy scalars by value",
        explanation: r#"
A readonly borrow of a machine scalar adds indirection to a value that Copy can pass directly.
Instead, you SHOULD pass the scalar by value and rely on its intrinsic Copy behavior.
"#,
        example: {
            reported: r#"
function increment(value: &readonly int32): int32 {
    return value + 1;
}
"#,
            accepted: r#"
function increment(value: int32): int32 {
    return value + 1;
}
"#,
        },
        provenance: [Clippy("trivially_copy_pass_by_ref")],
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report readonly parameters that borrow machine scalars.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect explicitly typed readonly parameter borrows
    for (parameter_id, parameter) in view.iter_nodes::<dir::Parameter>() {
        let Some(declared_type) = parameter.declared_type() else {
            continue;
        };
        if module.is_statically_absent(parameter_id.into_any()) {
            continue;
        }
        let dir::TypeExpression::BorrowedOf {
            lifetime,
            mutability,
            exclusivity,
            target_type,
            ..
        } = view.get(declared_type)
        else {
            continue;
        };
        let access = mutability.unwrap_or(dir::Mutability::Mutable).access();
        if access != dir::Access::Readonly || *exclusivity == Some(dir::Exclusivity::Exclusive) {
            continue;
        }
        if let Some(lifetime) = lifetime
            && module.parameter_lifetime_occurs_elsewhere(parameter_id, *lifetime)?
        {
            continue;
        }
        if module.return_type_borrows_parameter(parameter_id)? {
            continue;
        }

        // require one fixed-size compiler scalar
        let Some(primitive) = module.primitive_type(target_type.into_any())? else {
            continue;
        };
        let is_machine_scalar = match primitive {
            dir::PrimitiveType::Boolean | dir::PrimitiveType::Character => true,
            dir::PrimitiveType::Integer(dir::IntegerType::Pointer { .. }) => true,
            dir::PrimitiveType::Integer(dir::IntegerType::Fixed { width, .. }) => width <= 64,
            dir::PrimitiveType::Float(_) => true,
            dir::PrimitiveType::String | dir::PrimitiveType::Bigint => false,
        };
        if !is_machine_scalar {
            continue;
        }

        // replace the complete borrow with its retained value type
        let span = module.source_extent(declared_type.into_any())?;
        let mut diagnostic = lint.diagnostic("machine scalar is passed by readonly borrow", span);
        if lifetime.is_none()
            && let Some(suggestion) = suggestion(module, lint, declared_type, *target_type)?
        {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Replace one scalar borrow with its value type.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    borrowed: dir::LocalNodeId<dir::TypeExpression>,
    target: dir::LocalNodeId<dir::TypeExpression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(borrowed.into_any())?;
    let retained = module.source_extent(target.into_any())?;
    if module.has_unretained_comment(extent, &[retained])? {
        return Ok(None);
    }

    // retain the exact authored scalar type
    let target = module.source(retained)?;
    let patch = Patch::replace(extent, target);
    let suggestion = lint.suggestion("pass the Copy scalar by value", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace a readonly boolean borrow with its value type.
    #[test]
    fn test_replaces_readonly_boolean() {
        let session = TestSession::dir(
            &NEEDLESS_BORROW_OF_COPY,
            r#"
function negate(value: &readonly boolean): boolean {
    return !value;
}
"#,
        );

        session.assert_suggestions(
            r#"
function negate(value: boolean): boolean {
    return !value;
}
"#,
        );
    }

    /// Preserve scalar borrows that require mutation or exclusion.
    #[test]
    fn test_accepts_required_borrows() {
        let session = TestSession::dir(
            &NEEDLESS_BORROW_OF_COPY,
            r#"
function increment(value: &int32): void {
    *value += 1;
}

function read(value: &readonly exclusive int32): int32 {
    return *value;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve standard-library string borrows.
    #[test]
    fn test_accepts_string_borrow() {
        let session = TestSession::dir(
            &NEEDLESS_BORROW_OF_COPY,
            r#"
function length(value: &readonly string): isize {
    return value.length;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve readonly borrows of scalars wider than one machine register.
    #[test]
    fn test_accepts_wide_integer_borrow() {
        let session = TestSession::dir(
            &NEEDLESS_BORROW_OF_COPY,
            r#"
function equal(left: &readonly uint128, right: uint128): boolean {
    return left === right;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve a scalar borrow whose lifetime escapes through the return type.
    #[test]
    fn test_accepts_returned_borrow() {
        let session = TestSession::dir(
            &NEEDLESS_BORROW_OF_COPY,
            r#"
function identity<'a>(value: &'a readonly int32): &'a readonly int32 {
    return value;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve a scalar borrow whose lifetime is stored through another parameter.
    #[test]
    fn test_accepts_stored_borrow() {
        let session = TestSession::dir(
            &NEEDLESS_BORROW_OF_COPY,
            r#"
struct Holder<'a> {
    value: &'a readonly int32;
}

function store<'a>(holder: &Holder<'a>, value: &'a readonly int32): void {
    holder.value = value;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a readonly borrowed slice on a disabled method.
    #[test]
    fn test_accepts_disabled_borrowed_slice_parameter() {
        let session = TestSession::dir(
            &NEEDLESS_BORROW_OF_COPY,
            r#"
class NativeString {
    @if(false)
    static fromBytes(value: &readonly [uint8]): ^NativeString {
        // intentionally empty
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
