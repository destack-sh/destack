use tspp_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow wildcard arms when matching nominal enums.
    pub WILDCARD_ENUM_MATCH_ARM {
        id: "wildcard-enum-match-arm",
        summary: "Disallow wildcard arms when matching nominal enums",
        explanation: r#"
A wildcard arm accepts every future variant added to an enum without requiring the match to be revisited.
Instead, you SHOULD name every expected variant so exhaustiveness checking exposes additions.
"#,
        example: {
            reported: r#"
enum Mode {
    Read = 1,
    Write = 2,
}

function describe(mode: Mode): string {
    return match (mode) {
        Mode.Read => "read"
        _ => "other"
    };
}
"#,
            accepted: r#"
enum Mode {
    Read = 1,
    Write = 2,
}

function describe(mode: Mode): string {
    return match (mode) {
        Mode.Read => "read"
        Mode.Write => "write"
    };
}
"#,
        },
        provenance: [Clippy("wildcard_enum_match_arm")],
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report direct wildcard patterns in matches over nominal enums.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect matches whose reduced scrutinee type is one enum declaration
    for (_, expression) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::Match { value, arms } = expression else {
            continue;
        };
        let value_type = module.node_type_id(value.into_any())?;
        if !module.dir.is_enum_type(value_type)? {
            continue;
        }

        // report each wildcard in an unguarded top level arm pattern
        for arm in arms.iter().copied() {
            let (pattern, has_guard) = match view.get(arm) {
                dir::MatchArm::Expression { pattern, guard, .. }
                | dir::MatchArm::Block { pattern, guard, .. } => (*pattern, guard.is_some()),
            };
            if has_guard {
                continue;
            }

            // follow only transparent and union pattern structure
            let mut patterns = vec![pattern];
            while let Some(pattern) = patterns.pop() {
                match view.get(pattern) {
                    dir::Pattern::Wildcard => {
                        let span = module.source_extent(pattern.into_any())?;
                        let diagnostic =
                            lint.diagnostic("wildcard arm hides future enum variants", span);
                        output.report(diagnostic);
                    }
                    dir::Pattern::Must(pattern)
                    | dir::Pattern::DereferenceOf { right: pattern }
                    | dir::Pattern::BorrowOf { right: pattern, .. }
                    | dir::Pattern::MoveOf { right: pattern, .. }
                    | dir::Pattern::Default { pattern, .. }
                    | dir::Pattern::Binding {
                        pattern: Some(pattern),
                        ..
                    } => patterns.push(*pattern),
                    dir::Pattern::Union { patterns: nested } => {
                        patterns.extend(nested.iter().copied());
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report a wildcard arm over an enum value.
    #[test]
    fn test_reports_enum_wildcard() {
        let session = TestSession::dir(
            &WILDCARD_ENUM_MATCH_ARM,
            r#"
enum Mode {
    Read = 1,
    Write = 2,
}

function describe(mode: Mode): string {
    return match (mode) {
        Mode.Read => "read"
        _ => "other"
    };
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[wildcard-enum-match-arm]: wildcard arm hides future enum variants
  ──▶ main.tspp:9:9
   │
 7 │     return match (mode) {
 8 │         Mode.Read => "read"
 9 │         _ => "other"
   │         ^
10 │     };
11 │ }
   │
"#,
        );
    }

    /// Report a wildcard when the enum is accessed through a readonly borrow.
    #[test]
    fn test_reports_borrowed_enum_wildcard() {
        let session = TestSession::dir(
            &WILDCARD_ENUM_MATCH_ARM,
            r#"
enum Mode {
    Read = 1,
    Write = 2,
}

function describe(mode: &readonly Mode): string {
    return match (mode) {
        Mode.Read => "read"
        _ => "other"
    };
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[wildcard-enum-match-arm]: wildcard arm hides future enum variants
  ──▶ main.tspp:9:9
   │
 7 │     return match (mode) {
 8 │         Mode.Read => "read"
 9 │         _ => "other"
   │         ^
10 │     };
11 │ }
   │
"#,
        );
    }

    /// Report a wildcard nested in a top level union pattern.
    #[test]
    fn test_reports_union_wildcard() {
        let session = TestSession::dir(
            &WILDCARD_ENUM_MATCH_ARM,
            r#"
enum Mode {
    Read = 1,
    Write = 2,
}

function describe(mode: Mode): string {
    return match (mode) {
        Mode.Read | _ => "selected"
    };
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[wildcard-enum-match-arm]: wildcard arm hides future enum variants
  ──▶ main.tspp:8:21
   │
 6 │ function describe(mode: Mode): string {
 7 │     return match (mode) {
 8 │         Mode.Read | _ => "selected"
   │                     ^
 9 │     };
10 │ }
   │
"#,
        );
    }

    /// Accept an exhaustive enum match with explicit arms.
    #[test]
    fn test_accepts_explicit_enum_arms() {
        let session = TestSession::dir(
            &WILDCARD_ENUM_MATCH_ARM,
            r#"
enum Mode {
    Read = 1,
    Write = 2,
}

function describe(mode: Mode): string {
    return match (mode) {
        Mode.Read => "read"
        Mode.Write => "write"
    };
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept wildcard matching over a non-enum value.
    #[test]
    fn test_accepts_non_enum_wildcard() {
        let session = TestSession::dir(
            &WILDCARD_ENUM_MATCH_ARM,
            r#"
function describe(value: int32): string {
    return match (value) {
        0 => "zero"
        _ => "other"
    };
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a guarded wildcard that does not hide enum additions.
    #[test]
    fn test_accepts_guarded_enum_wildcard() {
        let session = TestSession::dir(
            &WILDCARD_ENUM_MATCH_ARM,
            r#"
enum Mode {
    Read = 1,
    Write = 2,
}

function describe(mode: Mode, isFallback: boolean): string {
    return match (mode) {
        _ if (isFallback) => "fallback"
        Mode.Read => "read"
        Mode.Write => "write"
    };
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
