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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Person {
    name: string;
    age: int32;
}

declare const person: Readonly<Person>;

person.age satisfies int32;

=== checked ===
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
/// @type.symbol symbol=person source=person type=Readonly<Person>
/// @resolution.name source=Readonly target=types.object.Readonly
/// @resolution.name source=Person target=Person

person.age satisfies int32;
/// @resolution.name source=person target=person
/// @resolution.member source=person.age receiver=Readonly<Person> kind=symbol target=Person.age

/// @generic.instance id=Readonly<Person> template=types.object.Readonly arguments=(Person)
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Person {
    name?: string;
}

const person: Readonly<Person> = {};

person.name satisfies string | undefined;

=== checked ===
interface Person {
/// @type.symbol symbol=Person type=Person
/// @definition.interface symbol=Person
/// @definition.field symbol=Person.name source="name?: string" key=name type=string

    name?: string;
    /// @type.symbol symbol=Person.name source="name?: string" type=string

}

const person: Readonly<Person> = {};
/// @type.symbol symbol=person source=person type=Readonly<Person>
/// @resolution.name source=Readonly target=types.object.Readonly
/// @resolution.name source=Person target=Person

person.name satisfies string | undefined;
/// @resolution.name source=person target=person
/// @resolution.member source=person.name receiver=Readonly<Person> kind=symbol target=Person.name

/// @generic.instance id=Readonly<Person> template=types.object.Readonly arguments=(Person)
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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Person {
    name: string;
    age: int32;
}

declare const person: Readonly<Person>;

person.name = "Grace";

=== checked ===
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
/// @type.symbol symbol=person source=person type=Readonly<Person>
/// @resolution.name source=Readonly target=types.object.Readonly
/// @resolution.name source=Person target=Person

person.name = "Grace";
/// @resolution.name source=person target=person
/// @resolution.pattern.assign source=person.name kind=place place=field(Person.name) type=string

/// @generic.instance id=Readonly<Person> template=types.object.Readonly arguments=(Person)
"#,
        r#"
/// @diagnostic.error code=EC214 message="cannot assign to readonly member 'name'"
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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Person {
    profile: {
        name: string;
    };
}

const person: Readonly<Person> = { profile: { name: "Ada" } };

person.profile.name = "Grace";

=== checked ===
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
/// @type.symbol symbol=person source=person type=Readonly<Person>
/// @resolution.name source=Readonly target=types.object.Readonly
/// @resolution.name source=Person target=Person

person.profile.name = "Grace";
/// @resolution.name source=person target=person
/// @resolution.member source=person.profile receiver=Readonly<Person> kind=symbol target=Person.profile
/// @resolution.pattern.assign source=person.profile.name kind=place place=field(name) type=string

/// @generic.instance id=Readonly<Person> template=types.object.Readonly arguments=(Person)
"#,
        r#"
/// @diagnostic.error code=EC214 message="cannot assign to readonly member 'name'"
/// @diagnostic.label line=10 column=16 span="name" line_source="person.profile.name = \"Grace\";"
"#,
    );
}
