use crate::tests::TestPattern;

/// Reject sequences that do not occupy repeated DIR fields.
#[test]
fn test_reject_sequence_root() {
    TestPattern::new("$$$VALUES").compile().assert_diagnostics(
        r#"
error[invalid-repeated-metavariable]: repeated metavariable does not occupy a repeated DIR list
 ──▶ tspp:pattern:1:1
  │
1 │ $$$VALUES
  │ ^^^^^^^^^
  │

for more information about an error, run `tspp explain invalid-repeated-metavariable`
"#,
    );
}

/// Reject repeated names with incompatible structural kinds.
#[test]
fn test_reject_incompatible_uses() {
    TestPattern::new("$VALUE.$VALUE")
        .compile()
        .assert_diagnostics(
            r#"
error[incompatible-pattern-metavariable]: metavariable 'VALUE' has incompatible uses
 ──▶ tspp:pattern:1:8
  │
1 │ $VALUE.$VALUE
  │ ------ ^^^^^^ first used here
  │

for more information about an error, run `tspp explain incompatible-pattern-metavariable`
"#,
        );
}

/// Reject adjacent sequences whose candidate partition would be ambiguous.
#[test]
fn test_reject_adjacent_sequences() {
    TestPattern::new("consume($$$LEFT, $$$RIGHT)")
        .compile()
        .assert_diagnostics(
            r#"
error[ambiguous-repeated-metavariables]: adjacent repeated metavariables have no unique partition
 ──▶ tspp:pattern:1:18
  │
1 │ consume($$$LEFT, $$$RIGHT)
  │                  ^^^^^^^^
  │

for more information about an error, run `tspp explain ambiguous-repeated-metavariables`
"#,
        );
}

/// Preserve strict fragment root-count failures.
#[test]
fn test_reject_multiple_roots() {
    TestPattern::new("first\nsecond")
        .compile()
        .assert_diagnostics(
            r#"
error[expected-pattern-root]: expected one pattern root, found 2
  ──▶ <pattern>
for more information about an error, run `tspp explain expected-pattern-root`
"#,
        );
}

/// Preserve authoritative parser diagnostics without translating their ids.
#[test]
fn test_preserve_parser_diagnostics() {
    TestPattern::new("fetch(").compile().assert_diagnostics(
        r#"
error[expected-expression]: expected expression
 ──▶ tspp:pattern:1:7
  │
1 │ fetch(
  │       ^ expected expression, found End
  │

for more information about an error, run `tspp explain expected-expression`
"#,
    );
}
