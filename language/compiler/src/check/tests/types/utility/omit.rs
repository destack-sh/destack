use crate::tests::{DirRows, TestSession};

#[test]
fn test_omit_removes_object_keys() {
    let session = TestSession::single(
        r#"
interface Person {
    name: string;
    age: int32;
    active: boolean;
}

declare const person: Omit<Person, "age">;

person.name satisfies string;
person.active satisfies boolean;
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
    active: boolean;
}

declare const person: Omit<Person, "age">;

person.name satisfies string;
person.active satisfies boolean;

=== checked ===
interface Person {
/// @type.symbol symbol=Person type=Person
/// @definition.interface symbol=Person
/// @definition.field symbol=Person.name source="name: string" key=name type=string
/// @definition.field symbol=Person.age source="age: int32" key=age type=int32
/// @definition.field symbol=Person.active source="active: boolean" key=active type=boolean

    name: string;
    /// @type.symbol symbol=Person.name source="name: string" type=string

    age: int32;
    /// @type.symbol symbol=Person.age source="age: int32" type=int32

    active: boolean;
    /// @type.symbol symbol=Person.active source="active: boolean" type=boolean

}

declare const person: Omit<Person, "age">;
/// @type.symbol symbol=person source=person type={ name: string; active: boolean }
/// @resolution.name source=Omit target=types.object.Omit
/// @resolution.name source=Person target=Person

person.name satisfies string;
/// @resolution.name source=person target=person
/// @resolution.member source=person.name receiver=Omit<Person, "age"> kind=field key=name

person.active satisfies boolean;
/// @resolution.name source=person target=person
/// @resolution.member source=person.active receiver=Omit<Person, "age"> kind=field key=active
"#,
    );
}

#[test]
fn test_omit_rejects_removed_keys() {
    let session = TestSession::single(
        r#"
interface Person {
    name: string;
    age: int32;
    active: boolean;
}

declare const person: Omit<Person, "age">;
const age = person.age;
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
    active: boolean;
}

declare const person: Omit<Person, "age">;
const age = person.age;

=== checked ===
interface Person {
/// @type.symbol symbol=Person type=Person
/// @definition.interface symbol=Person
/// @definition.field symbol=Person.name source="name: string" key=name type=string
/// @definition.field symbol=Person.age source="age: int32" key=age type=int32
/// @definition.field symbol=Person.active source="active: boolean" key=active type=boolean

    name: string;
    /// @type.symbol symbol=Person.name source="name: string" type=string

    age: int32;
    /// @type.symbol symbol=Person.age source="age: int32" type=int32

    active: boolean;
    /// @type.symbol symbol=Person.active source="active: boolean" type=boolean

}

declare const person: Omit<Person, "age">;
/// @type.symbol symbol=person source=person type={ name: string; active: boolean }
/// @resolution.name source=Omit target=types.object.Omit
/// @resolution.name source=Person target=Person

const age = person.age;
/// @type.symbol symbol=age type=<error>
/// @resolution.name source=person target=person
"#,
        r#"
/// @diagnostic.error code=EC300 message="member 'age' does not exist on type 'Omit<Person, \"age\">'"
/// @diagnostic.label line=9 column=13 source="const age = person.age;"
"#,
    );
}

#[test]
fn test_omit_accepts_remaining_field() {
    let session = TestSession::single(
        r#"
interface Person {
    name: string;
    age: int32;
}

type WithoutAge = Omit<Person, "age">;

const person: WithoutAge = { name: "Ada" };
person satisfies WithoutAge;
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

type WithoutAge = Omit<Person, "age">;

const person: WithoutAge = { name: "Ada" };
person satisfies WithoutAge;

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

type WithoutAge = Omit<Person, "age">;
/// @type.symbol symbol=WithoutAge source="type WithoutAge = Omit<Person, \"age\">" type={ name: string }
/// @definition.type symbol=WithoutAge source="type WithoutAge = Omit<Person, \"age\">" value={ name: string }
/// @resolution.name source=Omit target=types.object.Omit
/// @resolution.name source=Person target=Person

const person: WithoutAge = { name: "Ada" };
/// @type.symbol symbol=person source=person type={ name: string }
/// @resolution.name source=WithoutAge target=WithoutAge

person satisfies WithoutAge;
/// @resolution.name source=person target=person
/// @resolution.name source=WithoutAge target=WithoutAge
"#,
    );
}

#[test]
fn test_omit_rejects_removed_field_in_object_literal() {
    let session = TestSession::single(
        r#"
interface Person {
    name: string;
    age: int32;
}

type WithoutAge = Omit<Person, "age">;

const person: WithoutAge = { name: "Ada", age: 42 };
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

type WithoutAge = Omit<Person, "age">;

const person: WithoutAge = { name: "Ada", age: 42 };

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

type WithoutAge = Omit<Person, "age">;
/// @type.symbol symbol=WithoutAge source="type WithoutAge = Omit<Person, \"age\">" type={ name: string }
/// @definition.type symbol=WithoutAge source="type WithoutAge = Omit<Person, \"age\">" value={ name: string }
/// @resolution.name source=Omit target=types.object.Omit
/// @resolution.name source=Person target=Person

const person: WithoutAge = { name: "Ada", age: 42 };
/// @type.symbol symbol=person source=person type={ name: string }
/// @resolution.name source=WithoutAge target=WithoutAge
"#,
        r#"
/// @diagnostic.error code=EC205 message="unknown property 'age' in object literal for type '{ name: string }'"
/// @diagnostic.label line=9 column=44 source="const person: WithoutAge = { name: \"Ada\", age: 42 };"
"#,
    );
}

#[test]
fn test_omit_key_union_removes_all_fields() {
    let session = TestSession::single(
        r#"
interface Person {
    name: string;
    age: int32;
}

type WithoutAll = Omit<Person, "name" | "age">;

const person: WithoutAll = {};
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

type WithoutAll = Omit<Person, "name" | "age">;

const person: WithoutAll = {};

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

type WithoutAll = Omit<Person, "name" | "age">;
/// @type.symbol symbol=WithoutAll source="type WithoutAll = Omit<Person, \"name\" | \"age\">" type={}
/// @definition.type symbol=WithoutAll source="type WithoutAll = Omit<Person, \"name\" | \"age\">" value={}
/// @resolution.name source=Omit target=types.object.Omit
/// @resolution.name source=Person target=Person

const person: WithoutAll = {};
/// @type.symbol symbol=person source=person type={}
/// @resolution.name source=WithoutAll target=WithoutAll
"#,
    );
}

#[test]
fn test_omit_unknown_key_keeps_source_shape() {
    let session = TestSession::single(
        r#"
interface Person {
    name: string;
    age: int32;
}

type Same = Omit<Person, "missing">;

const person: Same = { name: "Ada", age: 42 };
person satisfies Person;
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

type Same = Omit<Person, "missing">;

const person: Same = { name: "Ada", age: 42 };
person satisfies Person;

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

type Same = Omit<Person, "missing">;
/// @type.symbol symbol=Same source="type Same = Omit<Person, \"missing\">" type={ name: string; age: int32 }
/// @definition.type symbol=Same source="type Same = Omit<Person, \"missing\">" value={ name: string; age: int32 }
/// @resolution.name source=Omit target=types.object.Omit
/// @resolution.name source=Person target=Person

const person: Same = { name: "Ada", age: 42 };
/// @type.symbol symbol=person source=person type={ name: string; age: int32 }
/// @resolution.name source=Same target=Same

person satisfies Person;
/// @resolution.name source=person target=person
/// @resolution.name source=Person target=Person
"#,
    );
}

#[test]
fn test_omit_preserves_readonly_field() {
    let session = TestSession::single(
        r#"
interface Person {
    readonly name: string;
    age: int32;
}

type NameOnly = Omit<Person, "age">;

const person: NameOnly = { name: "Ada" };
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

type NameOnly = Omit<Person, "age">;

const person: NameOnly = { name: "Ada" };
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

type NameOnly = Omit<Person, "age">;
/// @type.symbol symbol=NameOnly source="type NameOnly = Omit<Person, \"age\">" type={ readonly name: string }
/// @definition.type symbol=NameOnly source="type NameOnly = Omit<Person, \"age\">" value={ readonly name: string }
/// @resolution.name source=Omit target=types.object.Omit
/// @resolution.name source=Person target=Person

const person: NameOnly = { name: "Ada" };
/// @type.symbol symbol=person source=person type={ readonly name: string }
/// @resolution.name source=NameOnly target=NameOnly

person.name = "Grace";
/// @resolution.name source=person target=person
/// @resolution.member source=person.name receiver=Omit<Person, "age"> kind=field key=name
"#,
        r#"
/// @diagnostic.error code=EC214 message="cannot assign to readonly member 'name'"
/// @diagnostic.label line=10 column=1 source="person.name = \"Grace\";"
"#,
    );
}
