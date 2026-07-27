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

/// Resolve repeated markers across the directly authored list domains.
#[test]
fn test_compile_sequence_domains() {
    TestPattern::new("fetch<$$$VALUES>()").compile().assert(
        r#"
fetch<$$$VALUES>()
/// @pattern.root node=Expression source="fetch<$$$VALUES>()"
/// @pattern.metavariable name=VALUES kind=nodes node=GenericArgument
/// @pattern.use name=VALUES kind=nodes node=GenericArgument
"#,
    );
    TestPattern::new("match (value) { Point { $$$VALUES } => body }")
        .compile()
        .assert(
            r#"
match (value) { Point { $$$VALUES } => body }
/// @pattern.root node=Expression source="match (value) { Point { $$$VALUES } => body }"
/// @pattern.metavariable name=VALUES kind=nodes node=PatternField
/// @pattern.use name=VALUES kind=nodes node=PatternField
"#,
        );
    TestPattern::new("[$$$VALUES] = source").compile().assert(
        r#"
[$$$VALUES] = source
/// @pattern.root node=Expression source="[$$$VALUES] = source"
/// @pattern.metavariable name=VALUES kind=nodes node=AssignPatternField
/// @pattern.use name=VALUES kind=nodes node=AssignPatternField
"#,
    );
    TestPattern::new("match (value) { $$$ARMS }")
        .compile()
        .assert(
            r#"
match (value) { $$$ARMS }
/// @pattern.root node=Expression source="match (value) { $$$ARMS }"
/// @pattern.metavariable name=ARMS kind=nodes node=MatchArm
/// @pattern.use name=ARMS kind=nodes node=MatchArm
"#,
        );
    TestPattern::new("switch (value) { $$$CASES }")
        .compile()
        .assert(
            r#"
switch (value) { $$$CASES }
/// @pattern.root node=Expression source="switch (value) { $$$CASES }"
/// @pattern.metavariable name=CASES kind=nodes node=SwitchCase
/// @pattern.use name=CASES kind=nodes node=SwitchCase
"#,
        );
    TestPattern::new("type Values = [$$$ELEMENTS]")
        .compile()
        .assert(
            r#"
type Values = [$$$ELEMENTS]
/// @pattern.root node=Expression source="type Values = [$$$ELEMENTS]"
/// @pattern.metavariable name=ELEMENTS kind=nodes node=TupleElement
/// @pattern.use name=ELEMENTS kind=nodes node=TupleElement
"#,
        );
    TestPattern::new("type Values = { $$$MEMBERS }")
        .compile()
        .assert(
            r#"
type Values = { $$$MEMBERS }
/// @pattern.root node=Expression source="type Values = { $$$MEMBERS }"
/// @pattern.metavariable name=MEMBERS kind=nodes node=TypeMember
/// @pattern.use name=MEMBERS kind=nodes node=TypeMember
"#,
        );
    TestPattern::new("enum Values { $$$FIELDS }")
        .compile()
        .assert(
            r#"
enum Values { $$$FIELDS }
/// @pattern.root node=Expression source="enum Values { $$$FIELDS }"
/// @pattern.metavariable name=FIELDS kind=nodes node=EnumField
/// @pattern.use name=FIELDS kind=nodes node=EnumField
"#,
        );
    TestPattern::new("<div>$$$CHILDREN</div>").compile().assert(
        r#"
<div>$$$CHILDREN</div>
/// @pattern.root node=Expression source="<div>$$$CHILDREN</div>"
/// @pattern.metavariable name=CHILDREN kind=nodes node=TreeChild
/// @pattern.use name=CHILDREN kind=nodes node=TreeChild
"#,
    );
    TestPattern::new(
        r#"
<div>
    $$$CHILDREN
</div>
"#,
    )
    .compile()
    .assert(
        r#"
<div>
/// @pattern.root node=Expression source="<div>\n    $$$CHILDREN\n</div>"
    $$$CHILDREN
    /// @pattern.metavariable name=CHILDREN kind=nodes node=TreeChild
    /// @pattern.use name=CHILDREN kind=nodes node=TreeChild
</div>
"#,
    );
    TestPattern::new("@$$$DECORATORS class Example {}")
        .compile()
        .assert(
            r#"
@$$$DECORATORS class Example {}
/// @pattern.root node=Expression source="@$$$DECORATORS class Example {}"
/// @pattern.metavariable name=DECORATORS kind=nodes node=Decorator
/// @pattern.use name=DECORATORS kind=nodes node=Decorator
"#,
        );
    TestPattern::new(r#"import { $$$ITEMS } from "module""#)
        .compile()
        .assert(
            r#"
import { $$$ITEMS } from "module"
/// @pattern.root node=Expression source="import { $$$ITEMS } from \"module\""
/// @pattern.metavariable name=ITEMS kind=nodes node=DependencyItem
/// @pattern.use name=ITEMS kind=nodes node=DependencyItem
"#,
        );
    TestPattern::new("let $$$DECLARATORS").compile().assert(
        r#"
let $$$DECLARATORS
/// @pattern.root node=Expression source="let $$$DECLARATORS"
/// @pattern.metavariable name=DECLARATORS kind=nodes node=Declarator
/// @pattern.use name=DECLARATORS kind=nodes node=Declarator
"#,
    );
    TestPattern::new("<div $$$ATTRIBUTES />").compile().assert(
        r#"
<div $$$ATTRIBUTES />
/// @pattern.root node=Expression source="<div $$$ATTRIBUTES />"
/// @pattern.metavariable name=ATTRIBUTES kind=nodes node=TreeAttribute
/// @pattern.use name=ATTRIBUTES kind=nodes node=TreeAttribute
"#,
    );
    TestPattern::new("function example<T>(): void where $$$CLAUSES {}")
        .compile()
        .assert(
            r#"
function example<T>(): void where $$$CLAUSES {}
/// @pattern.root node=Expression source="function example<T>(): void where $$$CLAUSES {}"
/// @pattern.metavariable name=CLAUSES kind=nodes node=WhereClause
/// @pattern.use name=CLAUSES kind=nodes node=WhereClause
"#,
        );
    TestPattern::new("function example(): void { $$$EXPRESSIONS }")
        .compile()
        .assert(
            r#"
function example(): void { $$$EXPRESSIONS }
/// @pattern.root node=Expression source="function example(): void { $$$EXPRESSIONS }"
/// @pattern.metavariable name=EXPRESSIONS kind=nodes node=Expression
/// @pattern.use name=EXPRESSIONS kind=nodes node=Expression
"#,
        );
    TestPattern::new("match (value) { $FIRST | $$$REST => body }")
        .compile()
        .assert(
            r#"
match (value) { $FIRST | $$$REST => body }
/// @pattern.root node=Expression source="match (value) { $FIRST | $$$REST => body }"
/// @pattern.metavariable name=FIRST kind=node node=Pattern
/// @pattern.use name=FIRST kind=node node=Pattern
/// @pattern.metavariable name=REST kind=nodes node=Pattern
/// @pattern.use name=REST kind=nodes node=Pattern
"#,
        );
}
