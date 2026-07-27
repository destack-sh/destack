use std::sync::Arc;

use destack_core::StringPool;

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
 ──▶ destack:replacement:1:8
  │
1 │ client($OTHER)
  │        ^^^^^^
  │

for more information about an error, run `destack explain unbound-replacement-metavariable`
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
 ──▶ destack:replacement:1:1
  │
1 │ $MEMBER
  │ ^^^^^^^
  │

 ──▶ destack:pattern:1:9
  │
1 │ $OBJECT.$MEMBER
  │         ------- first used here
  │

for more information about an error, run `destack explain incompatible-replacement-metavariable`
"#
        .trim_end()
    );
}
