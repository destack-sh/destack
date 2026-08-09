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
/// @definition.field symbol=Person.age source="age?: int32" key=age type=int32
/// @definition.field symbol=Person.name source="name: string" key=name type=string

    name: string;
    /// @type.symbol symbol=Person.name source="name: string" type=string

    age?: int32;
    /// @type.symbol symbol=Person.age source="age?: int32" type=int32

}

declare const person: Partial<Person>;
/// @type.symbol symbol=person source=person type=Partial<Person> reduced={ name?: string; age?: int32 }
/// @resolution.pattern source=person kind=binding target=person
/// @resolution.name source=Partial target=types.object.Partial
/// @resolution.name source=Person target=Person

person.name satisfies string | undefined;
/// @resolution.name source=person target=person
/// @resolution.member source=person.name receiver={ name?: string; age?: int32 } type=string | undefined kind=field target_receiver={ name?: string; age?: int32 } key=name target_type=string | undefined
/// @resolution.place source=person placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=person root=person
/// @resolution.place source=person.name placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=person.name root=person keys=[name]

person.age satisfies int32 | undefined;
/// @resolution.name source=person target=person
/// @resolution.member source=person.age receiver={ name?: string; age?: int32 } type=int32 | undefined kind=field target_receiver={ name?: string; age?: int32 } key=age target_type=int32 | undefined
/// @resolution.place source=person placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=person root=person
/// @resolution.place source=person.age placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=person.age root=person keys=[age]

/// @generic.instance id=Partial<Person> template=types.object.Partial arguments=(Person)
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
/// @type.symbol symbol=person source=person type=Partial<Person> reduced={ name?: string; age?: int32 }
/// @resolution.pattern source=person kind=binding target=person
/// @resolution.name source=Partial target=types.object.Partial
/// @resolution.name source=Person target=Person

person.name satisfies string | undefined;
/// @resolution.name source=person target=person
/// @resolution.member source=person.name receiver={ name?: string; age?: int32 } type=string | undefined kind=field target_receiver={ name?: string; age?: int32 } key=name target_type=string | undefined
/// @resolution.place source=person placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=person root=person
/// @resolution.place source=person.name placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=person.name root=person keys=[name]

/// @generic.instance id=Partial<Person> template=types.object.Partial arguments=(Person)
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
/// @type.symbol symbol=bad source=bad type=Partial<Person> reduced={ name?: string; age?: int32 }
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Partial target=types.object.Partial
/// @resolution.name source=Person target=Person

/// @generic.instance id=Partial<Person> template=types.object.Partial arguments=(Person)
"#,
        r#"
/// @diagnostic.error id=excess-property message="unknown property 'extra' in object literal for type 'Partial<Person>'"
/// @diagnostic.label line=7 column=30 span="{ name: \"Ada\", extra: true }" line_source="const bad: Partial<Person> = { name: \"Ada\", extra: true };"
/// @diagnostic.related line=7 column=12 span="Partial" line_source="const bad: Partial<Person> = { name: \"Ada\", extra: true };" message="expected due to this annotation"
/// @diagnostic.note message="object literals may only specify known properties"
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
/// @type.symbol symbol=bad source=bad type=Partial<Person> reduced={ name?: string; age?: int32 }
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Partial target=types.object.Partial
/// @resolution.name source=Person target=Person

/// @generic.instance id=Partial<Person> template=types.object.Partial arguments=(Person)
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"no\"' is not assignable to type 'int32'"
/// @diagnostic.label line=7 column=50 span="\"no\"" line_source="const bad: Partial<Person> = { name: \"Ada\", age: \"no\" };"
/// @diagnostic.related line=7 column=12 span="Partial" line_source="const bad: Partial<Person> = { name: \"Ada\", age: \"no\" };" message="expected due to this annotation"
/// @diagnostic.note message="the mismatch is in field 'age'"
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
/// @type.symbol symbol=person source=person type=Partial<Person> reduced={ readonly name?: string; age?: int32 }
/// @resolution.pattern source=person kind=binding target=person
/// @resolution.name source=Partial target=types.object.Partial
/// @resolution.name source=Person target=Person

person.name = "Grace";
/// @resolution.name source=person target=person
/// @resolution.place source=person placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=person root=person
/// @resolution.rejected source=person.name

/// @generic.instance id=Partial<Person> template=types.object.Partial arguments=(Person)
"#,
        r#"

"#,
    );
}
