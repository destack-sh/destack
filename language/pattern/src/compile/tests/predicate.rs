use crate::tests::TestPattern;

/// Compile boolean composition, symbol equality, and metavariable reads.
#[test]
fn test_compile_predicate_algebra() {
    TestPattern::new("$CALLEE($$$ARGUMENTS)")
        .predicate("$CALLEE == myPackage.net.fetch && $CALLEE != null")
        .compile()
        .assert(
            r#"
$CALLEE($$$ARGUMENTS)
/// @pattern.root node=Expression source="$CALLEE($$$ARGUMENTS)"
/// @pattern.metavariable name=CALLEE kind=node node=Expression
/// @pattern.use name=CALLEE kind=node node=Expression
/// @pattern.metavariable name=ARGUMENTS kind=nodes node=Argument
/// @pattern.use name=ARGUMENTS kind=nodes node=Argument

$CALLEE == myPackage.net.fetch && $CALLEE != null
/// @predicate.root node=Expression source="$CALLEE == myPackage.net.fetch && $CALLEE != null"
/// @predicate.use name=CALLEE node=Expression
/// @predicate.use name=CALLEE node=Expression
"#,
        );
}

/// Reject predicate references absent from the structural pattern.
#[test]
fn test_reject_unbound_predicate_metavariable() {
    TestPattern::new("$CALLEE($$$ARGUMENTS)")
        .predicate("$OTHER == myPackage.net.fetch")
        .compile()
        .assert_diagnostics(
            r#"
error[unbound-predicate-metavariable]: predicate metavariable 'OTHER' is not bound by the pattern
 ──▶ tspp:predicate:1:1
  │
1 │ $OTHER == myPackage.net.fetch
  │ ^^^^^^
  │

for more information about an error, run `tspp explain unbound-predicate-metavariable`
"#,
        );
}

/// Compose multiple predicate sources as conjunctions over one capture table.
#[test]
fn test_compile_predicate_conjunction() {
    TestPattern::new("$CALLEE($VALUE)")
        .predicate("$CALLEE == myPackage.net.fetch")
        .predicate("$VALUE satisfies string")
        .compile()
        .assert(
            r#"
$CALLEE($VALUE)
/// @pattern.root node=Expression source="$CALLEE($VALUE)"
/// @pattern.metavariable name=CALLEE kind=node node=Expression
/// @pattern.use name=CALLEE kind=node node=Expression
/// @pattern.metavariable name=VALUE kind=node node=Expression
/// @pattern.use name=VALUE kind=node node=Expression

$CALLEE == myPackage.net.fetch
/// @predicate.root node=Expression source="$CALLEE == myPackage.net.fetch"
/// @predicate.use name=CALLEE node=Expression

$VALUE satisfies string
/// @predicate.root node=Expression source="$VALUE satisfies string"
/// @predicate.use name=VALUE node=Expression
"#,
        );
}

/// Reject repeated metavariable syntax in predicates.
#[test]
fn test_reject_repeated_predicate_metavariable() {
    TestPattern::new("fetch($$$ARGUMENTS)")
        .predicate("$$$ARGUMENTS == null")
        .compile()
        .assert_diagnostics(
            r#"
error[repeated-predicate-metavariable]: predicate metavariables cannot use repeated marker syntax
 ──▶ tspp:predicate:1:1
  │
1 │ $$$ARGUMENTS == null
  │ ^^^^^^^^^^^^
  │

for more information about an error, run `tspp explain repeated-predicate-metavariable`
"#,
        );
}

/// Reject expressions outside the side-effect-free predicate algebra.
#[test]
fn test_reject_unsupported_predicate_expression() {
    TestPattern::new("$CALLEE($$$ARGUMENTS)")
        .predicate("inspect($CALLEE)")
        .compile()
        .assert_diagnostics(
            r#"
error[unsupported-predicate-expression]: expression is not part of the pattern predicate language
 ──▶ tspp:predicate:1:1
  │
1 │ inspect($CALLEE)
  │ ^^^^^^^^^^^^^^^^
  │

for more information about an error, run `tspp explain unsupported-predicate-expression`
"#,
        );
}

/// Reject supported operators applied to unsupported values.
#[test]
fn test_reject_invalid_predicate_operands() {
    TestPattern::new("$CALLEE($$$ARGUMENTS)")
        .predicate("$CALLEE == inspect()")
        .compile()
        .assert_diagnostics(
            r#"
error[invalid-predicate-operands]: operator '==' cannot accept these predicate values
 ──▶ tspp:predicate:1:1
  │
1 │ $CALLEE == inspect()
  │ ^^^^^^^^^^^^^^^^^^^^
  │

for more information about an error, run `tspp explain invalid-predicate-operands`
"#,
        );
}

/// Reject operators without a defined predicate evaluation rule.
#[test]
fn test_reject_unsupported_predicate_operator() {
    TestPattern::new("$CALLEE($$$ARGUMENTS)")
        .predicate("$CALLEE in fetch")
        .compile()
        .assert_diagnostics(
            r#"
error[unsupported-predicate-expression]: expression is not part of the pattern predicate language
 ──▶ tspp:predicate:1:1
  │
1 │ $CALLEE in fetch
  │ ^^^^^^^^^^^^^^^^
  │

for more information about an error, run `tspp explain unsupported-predicate-expression`
"#,
        );
}

/// Require one complete predicate expression.
#[test]
fn test_reject_multiple_predicate_roots() {
    TestPattern::new("$VALUE")
        .predicate("$VALUE\n$VALUE")
        .compile()
        .assert_diagnostics(
            r#"
error[expected-predicate-root]: expected one predicate expression, found 2
  ──▶ <predicate>
for more information about an error, run `tspp explain expected-predicate-root`
"#,
        );
}

/// Reject name metavariables used outside expression position.
#[test]
fn test_reject_name_predicate_metavariable() {
    TestPattern::new("$OBJECT.$MEMBER")
        .predicate("$OBJECT.$MEMBER == null")
        .compile()
        .assert_diagnostics(
            r#"
error[invalid-predicate-metavariable]: predicate metavariable must occupy an expression
 ──▶ tspp:predicate:1:9
  │
1 │ $OBJECT.$MEMBER == null
  │         ^^^^^^^
  │

for more information about an error, run `tspp explain invalid-predicate-metavariable`
"#,
        );
}
