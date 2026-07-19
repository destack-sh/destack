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

    session.assert_dir_checked(
        "main.ds",
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

=== checked ===
interface Person {
/// @type.symbol symbol=Person type=Person
/// @definition.interface symbol=Person
/// @definition.field symbol=Person.age source="age?: int32" key=age type=int32
/// @definition.field symbol=Person.name source="name: string" key=name type=string

    name: string;
    /// @type.symbol symbol=Person.name source="name: string" type=string

    age?: int32;
    /// @type.symbol symbol=Person.age source="age?: int32" type=int32

}

declare const person: Required<Person>;
/// @type.symbol symbol=person source=person type=Required<Person> reduced={ name: string; age: int32 }
/// @resolution.name source=Required target=types.object.Required
/// @resolution.name source=Person target=Person

person.name satisfies string;
/// @resolution.name source=person target=person
/// @resolution.member source=person.name receiver={ name: string; age: int32 } kind=field key=name

person.age satisfies int32;
/// @resolution.name source=person target=person
/// @resolution.member source=person.age receiver={ name: string; age: int32 } kind=field key=age

/// @generic.instance id=Required<Person> template=types.object.Required arguments=(Person)
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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Person {
    name: string;
    age?: int32;
}

const person: Required<Person> = { name: "Ada" };

=== checked ===
interface Person {
/// @type.symbol symbol=Person type=Person
/// @definition.interface symbol=Person
/// @definition.field symbol=Person.age source="age?: int32" key=age type=int32
/// @definition.field symbol=Person.name source="name: string" key=name type=string

    name: string;
    /// @type.symbol symbol=Person.name source="name: string" type=string

    age?: int32;
    /// @type.symbol symbol=Person.age source="age?: int32" type=int32

}

const person: Required<Person> = { name: "Ada" };
/// @type.symbol symbol=person source=person type=Required<Person> reduced={ name: string; age: int32 }
/// @resolution.name source=Required target=types.object.Required
/// @resolution.name source=Person target=Person

/// @generic.instance id=Required<Person> template=types.object.Required arguments=(Person)
"#,
        r#"
/// @diagnostic.error id=missing-required-property message="missing required property 'age' for type 'Required<Person>'"
/// @diagnostic.label line=7 column=34 span="{ name: \"Ada\" }" line_source="const person: Required<Person> = { name: \"Ada\" };"
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Person {
    name?: string | undefined;
}

const person: Required<Person> = { name: undefined as string | undefined };
person.name satisfies string | undefined;

=== checked ===
interface Person {
/// @type.symbol symbol=Person type=Person
/// @definition.interface symbol=Person
/// @definition.field symbol=Person.name source="name?: string | undefined" key=name type=string | undefined

    name?: string | undefined;
    /// @type.symbol symbol=Person.name source="name?: string | undefined" type=string | undefined

}

const person: Required<Person> = { name: undefined };
/// @type.symbol symbol=person source=person type=Required<Person> reduced={ name: string | undefined }
/// @resolution.name source=Required target=types.object.Required
/// @resolution.name source=Person target=Person

person.name satisfies string | undefined;
/// @resolution.name source=person target=person
/// @resolution.member source=person.name receiver={ name: string | undefined } kind=field key=name

/// @generic.instance id=Required<Person> template=types.object.Required arguments=(Person)
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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Person {
    readonly name?: string;
}

const person: Required<Person> = { name: "Ada" };
person.name = "Grace";

=== checked ===
interface Person {
/// @type.symbol symbol=Person type=Person
/// @definition.interface symbol=Person
/// @definition.field symbol=Person.name source="readonly name?: string" key=name type=string

    readonly name?: string;
    /// @type.symbol symbol=Person.name source="readonly name?: string" type=string

}

const person: Required<Person> = { name: "Ada" };
/// @type.symbol symbol=person source=person type=Required<Person> reduced={ readonly name: string }
/// @resolution.name source=Required target=types.object.Required
/// @resolution.name source=Person target=Person

person.name = "Grace";
/// @resolution.name source=person target=person
/// @resolution.pattern.assign source=person.name kind=place place=field(name) type=string

/// @generic.instance id=Required<Person> template=types.object.Required arguments=(Person)
"#,
        r#"
/// @diagnostic.error id=cannot-assign-readonly-member message="cannot assign to readonly member 'name'"
/// @diagnostic.label line=7 column=8 span="name" line_source="person.name = \"Grace\";"
"#,
    );
}
