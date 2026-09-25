use crate::tests::{DirRows, TestSession};

#[test]
fn test_required_removes_field_optionality() {
    let session = TestSession::single(
        r#"
interface Person {
    name: string;
    age?: int32;
}

declare const person: Required<Person>;

person.name satisfies string;
person.age satisfies int32;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
interface Person {
    name: string;
    age?: int32;
}

declare const person: Required<Person>;

person.name satisfies string;
person.age satisfies int32;

=== dir ===
interface Person {
/// @generic.template symbol=Person parameters=(this: Person)
/// @type.symbol symbol=Person type=Person
/// @definition.interface symbol=Person template=(this: Person)
/// @definition.where symbol=Person relation=satisfies left=this right=Person
/// @definition.field symbol=Person.age source="age?: int32" key=age type=int32
/// @definition.field symbol=Person.name source="name: string" key=name type=string

    name: string;
    /// @type.symbol symbol=Person.name source="name: string" type=string

    age?: int32;
    /// @type.symbol symbol=Person.age source="age?: int32" type=int32

}

declare const person: Required<Person>;
/// @type.symbol symbol=person source=person type=Required<Person>
/// @resolution.pattern source=person kind=binding target=person
/// @resolution.name source=Required target=Required
/// @resolution.name source=Person target=Person

person.name satisfies string;
/// @resolution.name source=person target=person
/// @resolution.member source=person.name receiver=Required<Person> type=string kind=field target_receiver=Required<Person> key=name target_type=string
/// @resolution.place source=person placement="local" lifetime="static" access="immutable"
/// @resolution.access source=person root=person
/// @resolution.place source=person.name placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=person.name root=person keys=[name]

person.age satisfies int32;
/// @resolution.name source=person target=person
/// @resolution.member source=person.age receiver=Required<Person> type=int32 kind=field target_receiver=Required<Person> key=age target_type=int32
/// @resolution.place source=person placement="local" lifetime="static" access="immutable"
/// @resolution.access source=person root=person
/// @resolution.place source=person.age placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=person.age root=person keys=[age]
"#,
    );
}

#[test]
fn test_required_rejects_missing_optional_source_field() {
    let session = TestSession::single(
        r#"
interface Person {
    name: string;
    age?: int32;
}

const person: Required<Person> = { name: "Ada" };
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
interface Person {
    name: string;
    age?: int32;
}

const person: Required<Person> = { name: "Ada" };

=== dir ===
interface Person {
/// @generic.template symbol=Person parameters=(this: Person)
/// @type.symbol symbol=Person type=Person
/// @definition.interface symbol=Person template=(this: Person)
/// @definition.where symbol=Person relation=satisfies left=this right=Person
/// @definition.field symbol=Person.age source="age?: int32" key=age type=int32
/// @definition.field symbol=Person.name source="name: string" key=name type=string

    name: string;
    /// @type.symbol symbol=Person.name source="name: string" type=string

    age?: int32;
    /// @type.symbol symbol=Person.age source="age?: int32" type=int32

}

const person: Required<Person> = { name: "Ada" };
/// @type.symbol symbol=person source=person type=Required<Person>
/// @resolution.pattern source=person kind=binding target=person
/// @resolution.name source=Required target=Required
/// @resolution.name source=Person target=Person
"#,
        r#"
/// @diagnostic.error id=missing-required-property message="missing required property 'age' for type 'Required<Person>'"
/// @diagnostic.label line=7 column=34 span="{ name: \"Ada\" }" line_source="const person: Required<Person> = { name: \"Ada\" };"
/// @diagnostic.related line=7 column=15 span="Required" line_source="const person: Required<Person> = { name: \"Ada\" };" message="expected due to this annotation"
"#,
    );
}

#[test]
fn test_required_keeps_explicit_undefined_field_type() {
    let session = TestSession::single(
        r#"
interface Person {
    name?: string | undefined;
}

const person: Required<Person> = { name: undefined };
person.name satisfies string | undefined;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
interface Person {
    name?: string | undefined;
}

const person: Required<Person> = { name: undefined as string | undefined };
person.name satisfies string | undefined;

=== dir ===
interface Person {
/// @generic.template symbol=Person parameters=(this: Person)
/// @type.symbol symbol=Person type=Person
/// @definition.interface symbol=Person template=(this: Person)
/// @definition.where symbol=Person relation=satisfies left=this right=Person
/// @definition.field symbol=Person.name source="name?: string | undefined" key=name type=string | undefined

    name?: string | undefined;
    /// @type.symbol symbol=Person.name source="name?: string | undefined" type=string | undefined

}

const person: Required<Person> = { name: undefined };
/// @type.symbol symbol=person source=person type=Required<Person>
/// @resolution.pattern source=person kind=binding target=person
/// @resolution.name source=Required target=Required
/// @resolution.name source=Person target=Person

person.name satisfies string | undefined;
/// @resolution.name source=person target=person
/// @resolution.member source=person.name receiver=Required<Person> type=string | undefined kind=field target_receiver=Required<Person> key=name target_type=string | undefined
/// @resolution.place source=person placement="local" lifetime="static" access="immutable"
/// @resolution.access source=person root=person
/// @resolution.place source=person.name placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=person.name root=person keys=[name]
"#,
    );
}

#[test]
fn test_required_preserves_readonly_field() {
    let session = TestSession::single(
        r#"
interface Person {
    readonly name?: string;
}

const person: Required<Person> = { name: "Ada" };
person.name = "Grace";
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
interface Person {
    readonly name?: string;
}

const person: Required<Person> = { name: "Ada" };
person.name = "Grace";

=== dir ===
interface Person {
/// @generic.template symbol=Person parameters=(this: Person)
/// @type.symbol symbol=Person type=Person
/// @definition.interface symbol=Person template=(this: Person)
/// @definition.where symbol=Person relation=satisfies left=this right=Person
/// @definition.field symbol=Person.name source="readonly name?: string" key=name type=string

    readonly name?: string;
    /// @type.symbol symbol=Person.name source="readonly name?: string" type=string

}

const person: Required<Person> = { name: "Ada" };
/// @type.symbol symbol=person source=person type=Required<Person>
/// @resolution.pattern source=person kind=binding target=person
/// @resolution.name source=Required target=Required
/// @resolution.name source=Person target=Person

person.name = "Grace";
/// @resolution.name source=person target=person
/// @resolution.place source=person placement="local" lifetime="static" access="immutable"
/// @resolution.access source=person root=person
/// @resolution.rejected source=person.name
"#,
        r#"
/// @diagnostic.error id=cannot-assign-readonly-member message="cannot assign to readonly member 'name'"
/// @diagnostic.label line=7 column=8 span="name" line_source="person.name = \"Grace\";"
"#,
    );
}
