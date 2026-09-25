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

    session.assert_dir(
        "main.tspp",
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

=== dir ===
interface Person {
/// @generic.template symbol=Person parameters=(this: Person)
/// @type.symbol symbol=Person type=Person
/// @definition.interface symbol=Person template=(this: Person)
/// @definition.where symbol=Person relation=satisfies left=this right=Person
/// @definition.field symbol=Person.active source="active: boolean" key=active type=boolean
/// @definition.field symbol=Person.age source="age: int32" key=age type=int32
/// @definition.field symbol=Person.name source="name: string" key=name type=string

    name: string;
    /// @type.symbol symbol=Person.name source="name: string" type=string

    age: int32;
    /// @type.symbol symbol=Person.age source="age: int32" type=int32

    active: boolean;
    /// @type.symbol symbol=Person.active source="active: boolean" type=boolean

}

declare const person: Omit<Person, "age">;
/// @type.symbol symbol=person source=person type=Omit<Person, "age">
/// @resolution.pattern source=person kind=binding target=person
/// @resolution.name source=Omit target=Omit
/// @resolution.name source=Person target=Person

person.name satisfies string;
/// @resolution.name source=person target=person
/// @resolution.member source=person.name receiver=Omit<Person, "age"> type=string kind=field target_receiver=Omit<Person, "age"> key=name target_type=string
/// @resolution.place source=person placement="local" lifetime="static" access="immutable"
/// @resolution.access source=person root=person
/// @resolution.place source=person.name placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=person.name root=person keys=[name]

person.active satisfies boolean;
/// @resolution.name source=person target=person
/// @resolution.member source=person.active receiver=Omit<Person, "age"> type=boolean kind=field target_receiver=Omit<Person, "age"> key=active target_type=boolean
/// @resolution.place source=person placement="local" lifetime="static" access="immutable"
/// @resolution.access source=person root=person
/// @resolution.place source=person.active placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=person.active root=person keys=[active]
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

    session.assert_dir_and_diagnostics(
        "main.tspp",
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

=== dir ===
interface Person {
/// @generic.template symbol=Person parameters=(this: Person)
/// @type.symbol symbol=Person type=Person
/// @definition.interface symbol=Person template=(this: Person)
/// @definition.where symbol=Person relation=satisfies left=this right=Person
/// @definition.field symbol=Person.active source="active: boolean" key=active type=boolean
/// @definition.field symbol=Person.age source="age: int32" key=age type=int32
/// @definition.field symbol=Person.name source="name: string" key=name type=string

    name: string;
    /// @type.symbol symbol=Person.name source="name: string" type=string

    age: int32;
    /// @type.symbol symbol=Person.age source="age: int32" type=int32

    active: boolean;
    /// @type.symbol symbol=Person.active source="active: boolean" type=boolean

}

declare const person: Omit<Person, "age">;
/// @type.symbol symbol=person source=person type=Omit<Person, "age">
/// @resolution.pattern source=person kind=binding target=person
/// @resolution.name source=Omit target=Omit
/// @resolution.name source=Person target=Person

const age = person.age;
/// @type.symbol symbol=age source=age type=<error>
/// @resolution.pattern source=age kind=binding target=age
/// @resolution.name source=person target=person
/// @resolution.place source=person placement="local" lifetime="static" access="immutable"
/// @resolution.access source=person root=person
/// @resolution.rejected source=person.age
"#,
        r#"
/// @diagnostic.error id=missing-member message="member 'age' does not exist on type 'Omit<Person, \"age\">'"
/// @diagnostic.label line=9 column=20 span="age" line_source="const age = person.age;"
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

    session.assert_dir(
        "main.tspp",
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

type WithoutAge = Omit<Person, "age">;
/// @type.symbol symbol=WithoutAge source="type WithoutAge = Omit<Person, \"age\">" type={ name: string }
/// @definition.type symbol=WithoutAge source="type WithoutAge = Omit<Person, \"age\">" value=Omit<Person, "age">
/// @resolution.name source=Omit target=Omit
/// @resolution.name source=Person target=Person

const person: WithoutAge = { name: "Ada" };
/// @type.symbol symbol=person source=person type=WithoutAge
/// @resolution.pattern source=person kind=binding target=person
/// @resolution.name source=WithoutAge target=WithoutAge

person satisfies WithoutAge;
/// @resolution.name source=person target=person
/// @resolution.place source=person placement="local" lifetime="static" access="immutable"
/// @resolution.access source=person root=person
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

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
interface Person {
    name: string;
    age: int32;
}

type WithoutAge = Omit<Person, "age">;

const person: WithoutAge = { name: "Ada", age: 42 };

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

type WithoutAge = Omit<Person, "age">;
/// @type.symbol symbol=WithoutAge source="type WithoutAge = Omit<Person, \"age\">" type={ name: string }
/// @definition.type symbol=WithoutAge source="type WithoutAge = Omit<Person, \"age\">" value=Omit<Person, "age">
/// @resolution.name source=Omit target=Omit
/// @resolution.name source=Person target=Person

const person: WithoutAge = { name: "Ada", age: 42 };
/// @type.symbol symbol=person source=person type=WithoutAge
/// @resolution.pattern source=person kind=binding target=person
/// @resolution.name source=WithoutAge target=WithoutAge
"#,
        r#"
/// @diagnostic.error id=excess-property message="unknown property 'age' in object literal for type 'WithoutAge'"
/// @diagnostic.label line=9 column=28 span="{ name: \"Ada\", age: 42 }" line_source="const person: WithoutAge = { name: \"Ada\", age: 42 };"
/// @diagnostic.related line=9 column=15 span="WithoutAge" line_source="const person: WithoutAge = { name: \"Ada\", age: 42 };" message="expected due to this annotation"
/// @diagnostic.note message="object literals may only specify known properties"
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

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
interface Person {
    name: string;
    age: int32;
}

type WithoutAll = Omit<Person, "name" | "age">;

const person: WithoutAll = {};

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

type WithoutAll = Omit<Person, "name" | "age">;
/// @type.symbol symbol=WithoutAll source="type WithoutAll = Omit<Person, \"name\" | \"age\">" type={}
/// @definition.type symbol=WithoutAll source="type WithoutAll = Omit<Person, \"name\" | \"age\">" value=Omit<Person, "name" | "age">
/// @resolution.name source=Omit target=Omit
/// @resolution.name source=Person target=Person

const person: WithoutAll = {};
/// @type.symbol symbol=person source=person type=WithoutAll
/// @resolution.pattern source=person kind=binding target=person
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

    session.assert_dir(
        "main.tspp",
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

type Same = Omit<Person, "missing">;
/// @type.symbol symbol=Same source="type Same = Omit<Person, \"missing\">" type={ name: string; age: int32 }
/// @definition.type symbol=Same source="type Same = Omit<Person, \"missing\">" value=Omit<Person, "missing">
/// @resolution.name source=Omit target=Omit
/// @resolution.name source=Person target=Person

const person: Same = { name: "Ada", age: 42 };
/// @type.symbol symbol=person source=person type=Same
/// @resolution.pattern source=person kind=binding target=person
/// @resolution.name source=Same target=Same

person satisfies Person;
/// @resolution.name source=person target=person
/// @resolution.place source=person placement="local" lifetime="static" access="immutable"
/// @resolution.access source=person root=person
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

    session.assert_dir_and_diagnostics(
        "main.tspp",
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

=== dir ===
interface Person {
/// @generic.template symbol=Person parameters=(this: Person)
/// @type.symbol symbol=Person type=Person
/// @definition.interface symbol=Person template=(this: Person)
/// @definition.where symbol=Person relation=satisfies left=this right=Person
/// @definition.field symbol=Person.age source="age: int32" key=age type=int32
/// @definition.field symbol=Person.name source="readonly name: string" key=name type=string

    readonly name: string;
    /// @type.symbol symbol=Person.name source="readonly name: string" type=string

    age: int32;
    /// @type.symbol symbol=Person.age source="age: int32" type=int32

}

type NameOnly = Omit<Person, "age">;
/// @type.symbol symbol=NameOnly source="type NameOnly = Omit<Person, \"age\">" type={ readonly name: string }
/// @definition.type symbol=NameOnly source="type NameOnly = Omit<Person, \"age\">" value=Omit<Person, "age">
/// @resolution.name source=Omit target=Omit
/// @resolution.name source=Person target=Person

const person: NameOnly = { name: "Ada" };
/// @type.symbol symbol=person source=person type=NameOnly
/// @resolution.pattern source=person kind=binding target=person
/// @resolution.name source=NameOnly target=NameOnly

person.name = "Grace";
/// @resolution.name source=person target=person
/// @resolution.place source=person placement="local" lifetime="static" access="immutable"
/// @resolution.access source=person root=person
/// @resolution.rejected source=person.name
"#,
        r#"
/// @diagnostic.error id=cannot-assign-readonly-member message="cannot assign to readonly member 'name'"
/// @diagnostic.label line=10 column=8 span="name" line_source="person.name = \"Grace\";"
"#,
    );
}
