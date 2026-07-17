use crate::tests::{DirRows, TestSession};

#[test]
fn test_mutable_fields_removes_readonly_field_modifiers() {
    let session = TestSession::single(
        r#"
interface Person {
    readonly name: string;
    age: int32;
}

let person: MutableFields<Person> = { name: "Ada", age: 42 };

person.name = "Grace";
person.name satisfies string;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Person {
    readonly name: string;
    age: int32;
}

let person: MutableFields<Person> = { name: "Ada", age: 42 };

person.name = "Grace";
person.name satisfies string;

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

let person: MutableFields<Person> = { name: "Ada", age: 42 };
/// @type.symbol symbol=person source=person type=MutableFields<Person> reduced={ name: string; age: int32 }
/// @resolution.name source=MutableFields target=types.object.MutableFields
/// @resolution.name source=Person target=Person

person.name = "Grace";
/// @resolution.name source=person target=person
/// @resolution.pattern.assign source=person.name kind=place place=field(name) type=string

person.name satisfies string;
/// @resolution.name source=person target=person
/// @resolution.member source=person.name receiver={ name: string; age: int32 } kind=field key=name

/// @generic.instance id=MutableFields<Person> template=types.object.MutableFields arguments=(Person)
"#,
    );
}

#[test]
fn test_mutable_fields_preserves_optional_fields() {
    let session = TestSession::single(
        r#"
interface Person {
    readonly name?: string;
}

const empty: MutableFields<Person> = {};
const named: MutableFields<Person> = { name: "Ada" };

empty satisfies MutableFields<Person>;
named satisfies MutableFields<Person>;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Person {
    readonly name?: string;
}

const empty: MutableFields<Person> = {};
const named: MutableFields<Person> = { name: "Ada" };

empty satisfies MutableFields<Person>;
named satisfies MutableFields<Person>;

=== checked ===
interface Person {
/// @type.symbol symbol=Person type=Person
/// @definition.interface symbol=Person
/// @definition.field symbol=Person.name source="readonly name?: string" key=name type=string

    readonly name?: string;
    /// @type.symbol symbol=Person.name source="readonly name?: string" type=string

}

const empty: MutableFields<Person> = {};
/// @type.symbol symbol=empty source=empty type=MutableFields<Person> reduced={ name?: string }
/// @resolution.name source=MutableFields target=types.object.MutableFields
/// @resolution.name source=Person target=Person

const named: MutableFields<Person> = { name: "Ada" };
/// @type.symbol symbol=named source=named type=MutableFields<Person> reduced={ name?: string }
/// @resolution.name source=MutableFields target=types.object.MutableFields
/// @resolution.name source=Person target=Person

empty satisfies MutableFields<Person>;
/// @resolution.name source=empty target=empty
/// @resolution.name source=MutableFields target=types.object.MutableFields
/// @resolution.name source=Person target=Person

named satisfies MutableFields<Person>;
/// @resolution.name source=named target=named
/// @resolution.name source=MutableFields target=types.object.MutableFields
/// @resolution.name source=Person target=Person

/// @generic.instance id=MutableFields<Person> template=types.object.MutableFields arguments=(Person)
"#,
    );
}

#[test]
fn test_mutable_fields_keep_nested_readonly_views() {
    let session = TestSession::single(
        r#"
interface Person {
    readonly profile: readonly {
        name: string;
    };
}

let person: MutableFields<Person> = { profile: { name: "Ada" } };

person.profile.name = "Grace";
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Person {
    readonly profile: readonly {
        name: string;
    };
}

let person: MutableFields<Person> = { profile: { name: "Ada" } };

person.profile.name = "Grace";

=== checked ===
interface Person {
/// @type.symbol symbol=Person type=Person
/// @definition.interface symbol=Person
/// @definition.field symbol=Person.profile key=profile type=Readonly<{ name: string }>

    readonly profile: readonly {
    /// @type.symbol symbol=Person.profile type=Readonly<{ name: string }>

        name: string;
    };
}

let person: MutableFields<Person> = { profile: { name: "Ada" } };
/// @type.symbol symbol=person source=person type=MutableFields<Person> reduced={ profile: Readonly<{ name: string }> }
/// @resolution.name source=MutableFields target=types.object.MutableFields
/// @resolution.name source=Person target=Person

person.profile.name = "Grace";
/// @resolution.name source=person target=person
/// @resolution.member source=person.profile receiver={ profile: Readonly<{ name: string }> } kind=field key=profile
/// @resolution.pattern.assign source=person.profile.name kind=place place=field(name) type=string

/// @generic.instance id=MutableFields<Person> template=types.object.MutableFields arguments=(Person)
"#,
        r#"
/// @diagnostic.error code=EC214 message="cannot assign to readonly member 'name'"
/// @diagnostic.label line=10 column=16 span="name" line_source="person.profile.name = \"Grace\";"
"#,
    );
}
