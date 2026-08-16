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
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Person {
    name: string;
    age: int32;
}

declare const person: readonly Dynamic<Person>;

person.age satisfies int32;

=== dir ===
interface Person {
/// @type.symbol symbol=Person type=Person
/// @definition.interface symbol=Person
/// @definition.field symbol=Person.age source="age: int32" key=age type=int32
/// @definition.field symbol=Person.name source="name: string" key=name type=string

    name: string;
    /// @type.symbol symbol=Person.name source="name: string" type=string

    age: int32;
    /// @type.symbol symbol=Person.age source="age: int32" type=int32

}

declare const person: Readonly<Person>;
/// @type.symbol symbol=person source=person type=Readonly<Dynamic<Person>>
/// @resolution.pattern source=person kind=binding target=person
/// @resolution.name source=Readonly target=types.object.Readonly
/// @resolution.name source=Person target=Person

person.age satisfies int32;
/// @resolution.name source=person target=person
/// @resolution.member source=person.age receiver=Readonly<Dynamic<Person>> type=int32 kind=field target_receiver=Readonly<Dynamic<Person>> dispatch=dynamic constraint=Person key=age target=Person.age target_type=int32
/// @resolution.place source=person placement="local" lifetime="static" access="readonly"
/// @resolution.access source=person root=person
/// @resolution.place source=person.age placement="local" lifetime="static" access="readonly"
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
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Person {
    name?: string;
}

const person: readonly Dynamic<Person> = {} as readonly Dynamic<Person>;

person.name satisfies string | undefined;

=== dir ===
interface Person {
/// @type.symbol symbol=Person type=Person
/// @definition.interface symbol=Person
/// @definition.field symbol=Person.name source="name?: string" key=name type=string

    name?: string;
    /// @type.symbol symbol=Person.name source="name?: string" type=string

}

const person: Readonly<Person> = {};
/// @type.symbol symbol=person source=person type=Readonly<Dynamic<Person>>
/// @resolution.pattern source=person kind=binding target=person
/// @resolution.name source=Readonly target=types.object.Readonly
/// @resolution.name source=Person target=Person

person.name satisfies string | undefined;
/// @resolution.name source=person target=person
/// @resolution.member source=person.name receiver=Readonly<Dynamic<Person>> type=Readonly<string> | undefined kind=field target_receiver=Readonly<Dynamic<Person>> dispatch=dynamic constraint=Person key=name target=Person.name target_type=Readonly<string> | undefined
/// @resolution.place source=person placement="local" lifetime="static" access="readonly"
/// @resolution.access source=person root=person
/// @resolution.place source=person.name placement="local" lifetime="static" access="readonly"
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
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Person {
    name: string;
    age: int32;
}

declare const person: readonly Dynamic<Person>;

person.name = "Grace";

=== dir ===
interface Person {
/// @type.symbol symbol=Person type=Person
/// @definition.interface symbol=Person
/// @definition.field symbol=Person.age source="age: int32" key=age type=int32
/// @definition.field symbol=Person.name source="name: string" key=name type=string

    name: string;
    /// @type.symbol symbol=Person.name source="name: string" type=string

    age: int32;
    /// @type.symbol symbol=Person.age source="age: int32" type=int32

}

declare const person: Readonly<Person>;
/// @type.symbol symbol=person source=person type=Readonly<Dynamic<Person>>
/// @resolution.pattern source=person kind=binding target=person
/// @resolution.name source=Readonly target=types.object.Readonly
/// @resolution.name source=Person target=Person

person.name = "Grace";
/// @resolution.name source=person target=person
/// @resolution.place source=person placement="local" lifetime="static" access="readonly"
/// @resolution.access source=person root=person
/// @resolution.pattern.assign source=person.name kind=place
/// @resolution.access source=person.name root=person keys=[name]
/// @resolution.assignment source=person.name write="receiver=Readonly<Dynamic<Person>>, target=field(receiver=dynamic(Readonly<Dynamic<Person>>, constraint=Person), target=Person.name, type=Readonly<string>), type=Readonly<string>" type=Readonly<string>
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
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Person {
    profile: {
        name: string;
    };
}

const person: readonly Dynamic<Person> = { profile: { name: "Ada" } };

person.profile.name = "Grace";

=== dir ===
interface Person {
/// @type.symbol symbol=Person type=Person
/// @definition.interface symbol=Person
/// @definition.field symbol=Person.profile key=profile type={ name: string }

    profile: {
    /// @type.symbol symbol=Person.profile type={ name: string }

        name: string;
    };
}

const person: Readonly<Person> = { profile: { name: "Ada" } };
/// @type.symbol symbol=person source=person type=Readonly<Dynamic<Person>>
/// @resolution.pattern source=person kind=binding target=person
/// @resolution.name source=Readonly target=types.object.Readonly
/// @resolution.name source=Person target=Person

person.profile.name = "Grace";
/// @resolution.name source=person target=person
/// @resolution.member source=person.profile receiver=Readonly<Dynamic<Person>> type=Readonly<{ name: string }> kind=field target_receiver=Readonly<Dynamic<Person>> dispatch=dynamic constraint=Person key=profile target=Person.profile target_type=Readonly<{ name: string }>
/// @resolution.place source=person placement="local" lifetime="static" access="readonly"
/// @resolution.access source=person root=person
/// @resolution.place source=person.profile placement="local" lifetime="static" access="readonly"
/// @resolution.access source=person.profile root=person keys=[profile]
/// @resolution.pattern.assign source=person.profile.name kind=place
/// @resolution.access source=person.profile.name root=person keys=[profile, name]
/// @resolution.assignment source=person.profile.name write="receiver=Readonly<{ name: string }>, target=field(receiver=Readonly<{ name: string }>, target=name, type=Readonly<string>), type=Readonly<string>" type=Readonly<string>
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '{ profile: { name: \"Ada\" } }' is not assignable to type 'readonly Dynamic<Person>'"
/// @diagnostic.label line=8 column=34 span="{ profile: { name: \"Ada\" } }" line_source="const person: Readonly<Person> = { profile: { name: \"Ada\" } };"
/// @diagnostic.related line=8 column=15 span="Readonly" line_source="const person: Readonly<Person> = { profile: { name: \"Ada\" } };" message="expected due to this annotation"
/// @diagnostic.error id=cannot-assign-readonly-member message="cannot assign to readonly member 'name'"
/// @diagnostic.label line=10 column=16 span="name" line_source="person.profile.name = \"Grace\";"
"#,
    );
}
