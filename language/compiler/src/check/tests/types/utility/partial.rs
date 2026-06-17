use crate::tests::{DirRows, TestSession};

#[test]
fn test_partial_makes_fields_optional() {
    let session = TestSession::single(
        r#"
interface Person {
    name: string;
    age?: int32;
}

declare const person: Partial<Person>;

person.name satisfies string | undefined;
person.age satisfies int32 | undefined;
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

declare const person: Partial<Person>;

person.name satisfies string | undefined;
person.age satisfies int32 | undefined;

=== checked ===
interface Person {
/// @type.symbol symbol=Person type=Person
/// @definition.interface symbol=Person
/// @definition.field symbol=Person.age source="age?: int32" key=age type=int32 | undefined
/// @definition.field symbol=Person.name source="name: string" key=name type=string

    name: string;
    /// @type.symbol symbol=Person.name source="name: string" type=string

    age?: int32;
    /// @type.symbol symbol=Person.age source="age?: int32" type=int32 | undefined

}

declare const person: Partial<Person>;
/// @type.symbol symbol=person source=person type={ name?: string; age?: int32 }
/// @resolution.name source=Partial target=types.object.Partial
/// @resolution.name source=Person target=Person

person.name satisfies string | undefined;
/// @resolution.name source=person target=person
/// @resolution.member source=person.name receiver=types.object.Partial<Person> kind=field key=name

person.age satisfies int32 | undefined;
/// @resolution.name source=person target=person
/// @resolution.member source=person.age receiver=types.object.Partial<Person> kind=field key=age
"#,
    );
}

#[test]
fn test_partial_accepts_missing_required_fields() {
    let session = TestSession::single(
        r#"
interface Person {
    name: string;
    age: int32;
}

const person: Partial<Person> = {};

person.name satisfies string | undefined;
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

const person: Partial<Person> = {};

person.name satisfies string | undefined;

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

const person: Partial<Person> = {};
/// @type.symbol symbol=person source=person type={ name?: string; age?: int32 }
/// @resolution.name source=Partial target=types.object.Partial
/// @resolution.name source=Person target=Person

person.name satisfies string | undefined;
/// @resolution.name source=person target=person
/// @resolution.member source=person.name receiver=types.object.Partial<Person> kind=field key=name
"#,
    );
}

#[test]
fn test_partial_rejects_extra_field() {
    let session = TestSession::single(
        r#"
interface Person {
    name: string;
    age: int32;
}

const bad: Partial<Person> = { name: "Ada", extra: true };
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

const bad: Partial<Person> = { name: "Ada", extra: true };

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

const bad: Partial<Person> = { name: "Ada", extra: true };
/// @type.symbol symbol=bad source=bad type={ name?: string; age?: int32 }
/// @resolution.name source=Partial target=types.object.Partial
/// @resolution.name source=Person target=Person
"#,
        r#"
/// @diagnostic.error code=EC205 message="unknown property 'extra' in object literal for type '{ name?: string; age?: int32 }'"
/// @diagnostic.label line=7 column=45 source="const bad: Partial<Person> = { name: \"Ada\", extra: true };"
"#,
    );
}

#[test]
fn test_partial_rejects_incompatible_field_type() {
    let session = TestSession::single(
        r#"
interface Person {
    name: string;
    age: int32;
}

const bad: Partial<Person> = { name: "Ada", age: "no" };
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

const bad: Partial<Person> = { name: "Ada", age: "no" };

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

const bad: Partial<Person> = { name: "Ada", age: "no" };
/// @type.symbol symbol=bad source=bad type={ name?: string; age?: int32 }
/// @resolution.name source=Partial target=types.object.Partial
/// @resolution.name source=Person target=Person
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '\"no\"' is not assignable to type 'int32'"
/// @diagnostic.label line=7 column=52 source="\"no\""
"#,
    );
}

#[test]
fn test_partial_preserves_readonly_field() {
    let session = TestSession::single(
        r#"
interface Person {
    readonly name: string;
    age: int32;
}

const person: Partial<Person> = { name: "Ada" };
person.name = "Grace";
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Person {
    readonly name: string;
    age: int32;
}

const person: Partial<Person> = { name: "Ada" };
person.name = "Grace";

=== checked ===
interface Person {
/// @type.symbol symbol=Person type=Person
/// @definition.interface symbol=Person
/// @definition.field symbol=Person.age source="age: int32" key=age type=int32
/// @definition.field symbol=Person.name source="readonly name: string" key=name type=string

    readonly name: string;
    /// @type.symbol symbol=Person.name source="readonly name: string" type=string

    age: int32;
    /// @type.symbol symbol=Person.age source="age: int32" type=int32

}

const person: Partial<Person> = { name: "Ada" };
/// @type.symbol symbol=person source=person type={ readonly name?: string; age?: int32 }
/// @resolution.name source=Partial target=types.object.Partial
/// @resolution.name source=Person target=Person

person.name = "Grace";
/// @resolution.name source=person target=person
/// @resolution.member source=person.name receiver=types.object.Partial<Person> kind=field key=name
"#,
        r#"
/// @diagnostic.error code=EC214 message="cannot assign to readonly member 'name'"
/// @diagnostic.label line=8 column=1 source="person.name = \"Grace\";"
"#,
    );
}
