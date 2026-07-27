use crate::tests::TestPattern;

/// Lift a repeated call marker to its argument element.
#[test]
fn test_compile_argument_sequence() {
    TestPattern::new("fetch($URL, $$$ARGUMENTS)")
        .compile()
        .assert(
            r#"
fetch($URL, $$$ARGUMENTS)
/// @pattern.root node=Expression source="fetch($URL, $$$ARGUMENTS)"
/// @pattern.metavariable name=URL kind=node node=Expression
/// @pattern.use name=URL kind=node node=Expression
/// @pattern.metavariable name=ARGUMENTS kind=nodes node=Argument
/// @pattern.use name=ARGUMENTS kind=nodes node=Argument
"#,
        );
}

/// Lift repeated markers through distinct ordered DIR node families.
#[test]
fn test_compile_typed_sequences() {
    TestPattern::new("function collect<$$$GENERICS>($$$PARAMETERS): void {}")
        .compile()
        .assert(
            r#"
function collect<$$$GENERICS>($$$PARAMETERS): void {}
/// @pattern.root node=Expression source="function collect<$$$GENERICS>($$$PARAMETERS): void {}"
/// @pattern.metavariable name=GENERICS kind=nodes node=GenericParameter
/// @pattern.use name=GENERICS kind=nodes node=GenericParameter
/// @pattern.metavariable name=PARAMETERS kind=nodes node=Parameter
/// @pattern.use name=PARAMETERS kind=nodes node=Parameter
"#,
        );
    TestPattern::new("type Values = $FIRST | $$$REST")
        .compile()
        .assert(
            r#"
type Values = $FIRST | $$$REST
/// @pattern.root node=Expression source="type Values = $FIRST | $$$REST"
/// @pattern.metavariable name=FIRST kind=node node=TypeExpression
/// @pattern.use name=FIRST kind=node node=TypeExpression
/// @pattern.metavariable name=REST kind=nodes node=TypeExpression
/// @pattern.use name=REST kind=nodes node=TypeExpression
"#,
        );
    TestPattern::new("({ first: $FIRST, $$$REST })")
        .compile()
        .assert(
            r#"
({ first: $FIRST, $$$REST })
/// @pattern.root node=Expression source="{ first: $FIRST, $$$REST }"
/// @pattern.metavariable name=FIRST kind=node node=Expression
/// @pattern.use name=FIRST kind=node node=Expression
/// @pattern.metavariable name=REST kind=nodes node=Property
/// @pattern.use name=REST kind=nodes node=Property
"#,
        );
    TestPattern::new("class Example { $$$MEMBERS }")
        .compile()
        .assert(
            r#"
class Example { $$$MEMBERS }
/// @pattern.root node=Expression source="class Example { $$$MEMBERS }"
/// @pattern.metavariable name=MEMBERS kind=nodes node=Member
/// @pattern.use name=MEMBERS kind=nodes node=Member
"#,
        );
    TestPattern::new("[$FIRST, $$$REST] = $VALUE")
        .compile()
        .assert(
            r#"
[$FIRST, $$$REST] = $VALUE
/// @pattern.root node=Expression source="[$FIRST, $$$REST] = $VALUE"
/// @pattern.metavariable name=FIRST kind=node node=Expression
/// @pattern.use name=FIRST kind=node node=Expression
/// @pattern.metavariable name=REST kind=nodes node=AssignPatternField
/// @pattern.use name=REST kind=nodes node=AssignPatternField
/// @pattern.metavariable name=VALUE kind=node node=Expression
/// @pattern.use name=VALUE kind=node node=Expression
"#,
        );
}
