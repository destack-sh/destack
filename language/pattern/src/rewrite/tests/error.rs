use std::sync::Arc;

use tspp_core::StringPool;

use crate::Rewrite;
use crate::tests::{render_diagnostics, test_file};

/// Reject replacement names not bound by the search pattern.
#[test]
fn test_reject_unbound_replacement_metavariable() {
    let strings = Arc::new(StringPool::new());
    let pattern = test_file("<pattern>", "fetch($URL)");
    let replacement = test_file("<replacement>", "client($OTHER)");
    let diagnostics = Rewrite::parse(pattern, replacement.clone(), strings)
        .expect_err("unbound replacement should fail");
    let actual = render_diagnostics(&[replacement], &diagnostics);

    assert_eq!(
        actual,
        r#"error[unbound-replacement-metavariable]: replacement metavariable 'OTHER' is not bound by the pattern
 ──▶ tspp:replacement:1:8
  │
1 │ client($OTHER)
  │        ^^^^^^
  │

for more information about an error, run `tspp explain unbound-replacement-metavariable`
"#
        .trim_end()
    );
}

/// Reject replacement uses with a different structural value domain.
#[test]
fn test_reject_incompatible_replacement_metavariable() {
    let strings = Arc::new(StringPool::new());
    let pattern = test_file("<pattern>", "$OBJECT.$MEMBER");
    let replacement = test_file("<replacement>", "$MEMBER");
    let diagnostics = Rewrite::parse(pattern.clone(), replacement.clone(), strings)
        .expect_err("incompatible replacement should fail");
    let actual = render_diagnostics(&[pattern, replacement], &diagnostics);

    assert_eq!(
        actual,
        r#"error[incompatible-replacement-metavariable]: replacement metavariable 'MEMBER' has an incompatible use
 ──▶ tspp:replacement:1:1
  │
1 │ $MEMBER
  │ ^^^^^^^
  │

 ──▶ tspp:pattern:1:9
  │
1 │ $OBJECT.$MEMBER
  │         ------- first used here
  │

for more information about an error, run `tspp explain incompatible-replacement-metavariable`
"#
        .trim_end()
    );
}

/// Reject anonymous metavariables in replacement source.
#[test]
fn test_reject_anonymous_replacement_metavariable() {
    let strings = Arc::new(StringPool::new());
    let pattern = test_file("<pattern>", "fetch($_)");
    let replacement = test_file("<replacement>", "client($_)");
    let diagnostics = Rewrite::parse(pattern, replacement.clone(), strings)
        .expect_err("anonymous replacement should fail");
    let actual = render_diagnostics(&[replacement], &diagnostics);

    assert_eq!(
        actual,
        r#"error[anonymous-replacement-metavariable]: anonymous metavariables cannot be used in replacements
 ──▶ tspp:replacement:1:8
  │
1 │ client($_)
  │        ^^
  │

for more information about an error, run `tspp explain anonymous-replacement-metavariable`
"#
        .trim_end()
    );
}

/// Reject repeated replacement markers outside repeated DIR lists.
#[test]
fn test_reject_invalid_repeated_replacement_metavariable() {
    let strings = Arc::new(StringPool::new());
    let pattern = test_file("<pattern>", "fetch($$$ARGUMENTS)");
    let replacement = test_file("<replacement>", "$$$ARGUMENTS");
    let diagnostics = Rewrite::parse(pattern, replacement.clone(), strings)
        .expect_err("invalid repeated replacement should fail");
    let actual = render_diagnostics(&[replacement], &diagnostics);

    assert_eq!(
        actual,
        r#"error[invalid-repeated-replacement-metavariable]: repeated replacement metavariable does not occupy a repeated DIR list
 ──▶ tspp:replacement:1:1
  │
1 │ $$$ARGUMENTS
  │ ^^^^^^^^^^^^
  │

for more information about an error, run `tspp explain invalid-repeated-replacement-metavariable`
"#
        .trim_end()
    );
}
