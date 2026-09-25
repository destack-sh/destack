use crate::tests::{DirRows, TestSession};

#[test]
fn test_readonly_makes_fields_readonly() {
    let session = TestSession::single(
        r#"
interface Person {
    name: string;
    age: int32;
}

declare const person: Readonly<Person>;

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
    age: int32;
}

declare const person: readonly Person;

person.age satisfies int32;

=== dir ===
interface Person {
/// @generic.template symbol=Person parameters=(this: Person)
/// @type.symbol symbol=Person type=Person
/// @definition.interface symbol=Person template=(this: Person)
/// @definition.where symbol=Person relation=satisfies left=this right=Person
/// @definition.field symbol=Person.age source="age: int32" key=age type=int32
/// @definition.field symbol=Person.name source="name: string" key=name type=string

    name: string;
    /// @type.symbol symbol=Person.name source="name: string" type=string

    age: int32;
    /// @type.symbol symbol=Person.age source="age: int32" type=int32

}

declare const person: Readonly<Person>;
/// @type.symbol symbol=person source=person type=readonly Person
/// @resolution.pattern source=person kind=binding target=person
/// @resolution.name source=Readonly target=Readonly
/// @resolution.name source=Person target=Person

person.age satisfies int32;
/// @resolution.name source=person target=person
/// @resolution.member source=person.age receiver=readonly Person type=int32 kind=field target_receiver=readonly Person dispatch=dynamic constraint=Person key=age target=Person.age target_type=int32
/// @resolution.place source=person placement="local" lifetime="static" access="immutable"
/// @resolution.access source=person root=person
/// @resolution.place source=person.age placement="local" lifetime="managed" access="readonly"
/// @resolution.access source=person.age root=person keys=[age]
"#,
    );
}

#[test]
fn test_readonly_preserves_optional_fields() {
    let session = TestSession::single(
        r#"
interface Person {
    name?: string;
}

const person: Readonly<Person> = {};

person.name satisfies string | undefined;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
interface Person {
    name?: string;
}

const person: readonly Person = {} as readonly Person;

person.name satisfies string | undefined;

=== dir ===
interface Person {
/// @generic.template symbol=Person parameters=(this: Person)
/// @type.symbol symbol=Person type=Person
/// @definition.interface symbol=Person template=(this: Person)
/// @definition.where symbol=Person relation=satisfies left=this right=Person
/// @definition.field symbol=Person.name source="name?: string" key=name type=string

    name?: string;
    /// @type.symbol symbol=Person.name source="name?: string" type=string

}

const person: Readonly<Person> = {};
/// @type.symbol symbol=person source=person type=readonly Person
/// @resolution.pattern source=person kind=binding target=person
/// @resolution.name source=Readonly target=Readonly
/// @resolution.name source=Person target=Person

person.name satisfies string | undefined;
/// @resolution.name source=person target=person
/// @resolution.member source=person.name receiver=readonly Person type=string | undefined kind=field target_receiver=readonly Person dispatch=dynamic constraint=Person key=name target=Person.name target_type=string | undefined
/// @resolution.place source=person placement="local" lifetime="static" access="immutable"
/// @resolution.access source=person root=person
/// @resolution.place source=person.name placement="local" lifetime="managed" access="readonly"
/// @resolution.access source=person.name root=person keys=[name]
"#,
    );
}

#[test]
fn test_readonly_rejects_field_assignment() {
    let session = TestSession::single(
        r#"
interface Person {
    name: string;
    age: int32;
}

declare const person: Readonly<Person>;

person.name = "Grace";
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
interface Person {
    name: string;
    age: int32;
}

declare const person: readonly Person;

person.name = "Grace";

=== dir ===
interface Person {
/// @generic.template symbol=Person parameters=(this: Person)
/// @type.symbol symbol=Person type=Person
/// @definition.interface symbol=Person template=(this: Person)
/// @definition.where symbol=Person relation=satisfies left=this right=Person
/// @definition.field symbol=Person.age source="age: int32" key=age type=int32
/// @definition.field symbol=Person.name source="name: string" key=name type=string

    name: string;
    /// @type.symbol symbol=Person.name source="name: string" type=string

    age: int32;
    /// @type.symbol symbol=Person.age source="age: int32" type=int32

}

declare const person: Readonly<Person>;
/// @type.symbol symbol=person source=person type=readonly Person
/// @resolution.pattern source=person kind=binding target=person
/// @resolution.name source=Readonly target=Readonly
/// @resolution.name source=Person target=Person

person.name = "Grace";
/// @resolution.name source=person target=person
/// @resolution.place source=person placement="local" lifetime="static" access="immutable"
/// @resolution.access source=person root=person
/// @resolution.pattern.assign source=person.name kind=place
/// @resolution.place source=person.name placement="local" lifetime="managed" access="readonly"
/// @resolution.access source=person.name root=person keys=[name]
/// @resolution.assignment source=person.name write="receiver=readonly Person, target=field(receiver=dynamic(readonly Person, constraint=Person), target=Person.name, type=string), type=string" type=string
"#,
        r#"
/// @diagnostic.error id=cannot-assign-readonly-member message="cannot assign to readonly member 'name'"
/// @diagnostic.label line=9 column=8 span="name" line_source="person.name = \"Grace\";"
"#,
    );
}

#[test]
fn test_readonly_rejects_nested_field_assignment() {
    let session = TestSession::single(
        r#"
interface Person {
    profile: {
        name: string;
    };
}

const person: Readonly<Person> = { profile: { name: "Ada" } };

person.profile.name = "Grace";
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
interface Person {
    profile: {
        name: string;
    };
}

const person: readonly Person = { profile: { name: "Ada" } } as readonly Person;

person.profile.name = "Grace";

=== dir ===
interface Person {
/// @generic.template symbol=Person parameters=(this: Person)
/// @type.symbol symbol=Person type=Person
/// @definition.interface symbol=Person template=(this: Person)
/// @definition.where symbol=Person relation=satisfies left=this right=Person
/// @definition.field symbol=Person.profile key=profile type={ name: string }

    profile: {
    /// @type.symbol symbol=Person.profile type={ name: string }

        name: string;
        /// @type.symbol symbol=Person.name source="name: string" type=string

    };
}

const person: Readonly<Person> = { profile: { name: "Ada" } };
/// @type.symbol symbol=person source=person type=readonly Person
/// @resolution.pattern source=person kind=binding target=person
/// @resolution.name source=Readonly target=Readonly
/// @resolution.name source=Person target=Person

person.profile.name = "Grace";
/// @resolution.name source=person target=person
/// @resolution.member source=person.profile receiver=readonly Person type={ name: string } kind=field target_receiver=readonly Person dispatch=dynamic constraint=Person key=profile target=Person.profile target_type={ name: string }
/// @resolution.place source=person placement="local" lifetime="static" access="immutable"
/// @resolution.access source=person root=person
/// @resolution.place source=person.profile placement="local" lifetime="managed" access="readonly"
/// @resolution.access source=person.profile root=person keys=[profile]
/// @resolution.pattern.assign source=person.profile.name kind=place
/// @resolution.place source=person.profile.name placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=person.profile.name root=person keys=[profile, name]
/// @resolution.assignment source=person.profile.name write="receiver=readonly { name: string }, target=field(receiver=readonly { name: string }, target=name, type=string), type=string" type=string
"#,
        r#"
/// @diagnostic.error id=cannot-assign-readonly-member message="cannot assign to readonly member 'name'"
/// @diagnostic.label line=10 column=16 span="name" line_source="person.profile.name = \"Grace\";"
"#,
    );
}
