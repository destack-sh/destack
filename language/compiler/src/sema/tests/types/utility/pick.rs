use crate::tests::{DirRows, TestSession};

#[test]
fn test_pick_selects_object_keys() {
    let session = TestSession::single(
        r#"
interface Person {
    name: string;
    age: int32;
    active: boolean;
}

declare const person: Pick<Person, "name" | "active">;

person.name satisfies string;
person.active satisfies boolean;
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
    active: boolean;
}

declare const person: Pick<Person, "name" | "active">;

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

declare const person: Pick<Person, "name" | "active">;
/// @type.symbol symbol=person source=person type=Pick<Person, "name" | "active">
/// @resolution.pattern source=person kind=binding target=person
/// @resolution.name source=Pick target=Pick
/// @resolution.name source=Person target=Person

person.name satisfies string;
/// @resolution.name source=person target=person
/// @resolution.member source=person.name receiver=Pick<Person, "name" | "active"> type=string kind=field target_receiver=Pick<Person, "name" | "active"> key=name target_type=string
/// @resolution.place source=person placement="local" lifetime="static" access="immutable"
/// @resolution.access source=person root=person
/// @resolution.place source=person.name placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=person.name root=person keys=[name]

person.active satisfies boolean;
/// @resolution.name source=person target=person
/// @resolution.member source=person.active receiver=Pick<Person, "name" | "active"> type=boolean kind=field target_receiver=Pick<Person, "name" | "active"> key=active target_type=boolean
/// @resolution.place source=person placement="local" lifetime="static" access="immutable"
/// @resolution.access source=person root=person
/// @resolution.place source=person.active placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=person.active root=person keys=[active]
"#,
    );
}

#[test]
fn test_pick_rejects_unselected_keys() {
    let session = TestSession::single(
        r#"
interface Person {
    name: string;
    age: int32;
    active: boolean;
}

declare const person: Pick<Person, "name" | "active">;
const age = person.age;
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
    active: boolean;
}

declare const person: Pick<Person, "name" | "active">;
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

declare const person: Pick<Person, "name" | "active">;
/// @type.symbol symbol=person source=person type=Pick<Person, "name" | "active">
/// @resolution.pattern source=person kind=binding target=person
/// @resolution.name source=Pick target=Pick
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
/// @diagnostic.error id=missing-member message="member 'age' does not exist on type 'Pick<Person, \"name\" | \"active\">'"
/// @diagnostic.label line=9 column=20 span="age" line_source="const age = person.age;"
"#,
    );
}

#[test]
fn test_pick_preserves_optional_field() {
    let session = TestSession::single(
        r#"
interface Person {
    name: string;
    age?: int32;
}

type AgeOnly = Pick<Person, "age">;

const empty: AgeOnly = {};
const aged: AgeOnly = { age: 42 };

empty satisfies AgeOnly;
aged satisfies AgeOnly;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Person {
    name: string;
    age?: int32;
}

type AgeOnly = Pick<Person, "age">;

const empty: AgeOnly = {};
const aged: AgeOnly = { age: 42 as int32 | undefined };

empty satisfies AgeOnly;
aged satisfies AgeOnly;

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

type AgeOnly = Pick<Person, "age">;
/// @type.symbol symbol=AgeOnly source="type AgeOnly = Pick<Person, \"age\">" type={ age?: int32 }
/// @definition.type symbol=AgeOnly source="type AgeOnly = Pick<Person, \"age\">" value=Pick<Person, "age">
/// @resolution.name source=Pick target=Pick
/// @resolution.name source=Person target=Person

const empty: AgeOnly = {};
/// @type.symbol symbol=empty source=empty type=AgeOnly
/// @resolution.pattern source=empty kind=binding target=empty
/// @resolution.name source=AgeOnly target=AgeOnly

const aged: AgeOnly = { age: 42 };
/// @type.symbol symbol=aged source=aged type=AgeOnly
/// @resolution.pattern source=aged kind=binding target=aged
/// @resolution.name source=AgeOnly target=AgeOnly

empty satisfies AgeOnly;
/// @resolution.name source=empty target=empty
/// @resolution.place source=empty placement="local" lifetime="static" access="immutable"
/// @resolution.access source=empty root=empty
/// @resolution.name source=AgeOnly target=AgeOnly

aged satisfies AgeOnly;
/// @resolution.name source=aged target=aged
/// @resolution.place source=aged placement="local" lifetime="static" access="immutable"
/// @resolution.access source=aged root=aged
/// @resolution.name source=AgeOnly target=AgeOnly
"#,
    );
}

#[test]
fn test_pick_optional_field_rejects_extra_field() {
    let session = TestSession::single(
        r#"
interface Person {
    name: string;
    age?: int32;
}

type AgeOnly = Pick<Person, "age">;

const person: AgeOnly = { name: "Ada" };
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Person {
    name: string;
    age?: int32;
}

type AgeOnly = Pick<Person, "age">;

const person: AgeOnly = { name: "Ada" };

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

type AgeOnly = Pick<Person, "age">;
/// @type.symbol symbol=AgeOnly source="type AgeOnly = Pick<Person, \"age\">" type={ age?: int32 }
/// @definition.type symbol=AgeOnly source="type AgeOnly = Pick<Person, \"age\">" value=Pick<Person, "age">
/// @resolution.name source=Pick target=Pick
/// @resolution.name source=Person target=Person

const person: AgeOnly = { name: "Ada" };
/// @type.symbol symbol=person source=person type=AgeOnly
/// @resolution.pattern source=person kind=binding target=person
/// @resolution.name source=AgeOnly target=AgeOnly
"#,
        r#"
/// @diagnostic.error id=excess-property message="unknown property 'name' in object literal for type 'AgeOnly'"
/// @diagnostic.label line=9 column=25 span="{ name: \"Ada\" }" line_source="const person: AgeOnly = { name: \"Ada\" };"
/// @diagnostic.related line=9 column=15 span="AgeOnly" line_source="const person: AgeOnly = { name: \"Ada\" };" message="expected due to this annotation"
/// @diagnostic.note message="object literals may only specify known properties"
"#,
    );
}

#[test]
fn test_pick_accepts_required_field() {
    let session = TestSession::single(
        r#"
interface Person {
    name: string;
    age: int32;
}

type NameOnly = Pick<Person, "name">;

const person: NameOnly = { name: "Ada" };
person satisfies NameOnly;
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

type NameOnly = Pick<Person, "name">;

const person: NameOnly = { name: "Ada" };
person satisfies NameOnly;

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

type NameOnly = Pick<Person, "name">;
/// @type.symbol symbol=NameOnly source="type NameOnly = Pick<Person, \"name\">" type={ name: string }
/// @definition.type symbol=NameOnly source="type NameOnly = Pick<Person, \"name\">" value=Pick<Person, "name">
/// @resolution.name source=Pick target=Pick
/// @resolution.name source=Person target=Person

const person: NameOnly = { name: "Ada" };
/// @type.symbol symbol=person source=person type=NameOnly
/// @resolution.pattern source=person kind=binding target=person
/// @resolution.name source=NameOnly target=NameOnly

person satisfies NameOnly;
/// @resolution.name source=person target=person
/// @resolution.place source=person placement="local" lifetime="static" access="immutable"
/// @resolution.access source=person root=person
/// @resolution.name source=NameOnly target=NameOnly
"#,
    );
}

#[test]
fn test_pick_rejects_missing_required_field() {
    let session = TestSession::single(
        r#"
interface Person {
    name: string;
    age: int32;
}

type NameOnly = Pick<Person, "name">;

const person: NameOnly = {};
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

type NameOnly = Pick<Person, "name">;

const person: NameOnly = {};

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

type NameOnly = Pick<Person, "name">;
/// @type.symbol symbol=NameOnly source="type NameOnly = Pick<Person, \"name\">" type={ name: string }
/// @definition.type symbol=NameOnly source="type NameOnly = Pick<Person, \"name\">" value=Pick<Person, "name">
/// @resolution.name source=Pick target=Pick
/// @resolution.name source=Person target=Person

const person: NameOnly = {};
/// @type.symbol symbol=person source=person type=NameOnly
/// @resolution.pattern source=person kind=binding target=person
/// @resolution.name source=NameOnly target=NameOnly
"#,
        r#"
/// @diagnostic.error id=missing-required-property message="missing required property 'name' for type 'NameOnly'"
/// @diagnostic.label line=9 column=26 span="{}" line_source="const person: NameOnly = {};"
/// @diagnostic.related line=9 column=15 span="NameOnly" line_source="const person: NameOnly = {};" message="expected due to this annotation"
"#,
    );
}

#[test]
fn test_pick_rejects_extra_field() {
    let session = TestSession::single(
        r#"
interface Person {
    name: string;
    age: int32;
}

type NameOnly = Pick<Person, "name">;

const person: NameOnly = { name: "Ada", age: 42 };
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

type NameOnly = Pick<Person, "name">;

const person: NameOnly = { name: "Ada", age: 42 };

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

type NameOnly = Pick<Person, "name">;
/// @type.symbol symbol=NameOnly source="type NameOnly = Pick<Person, \"name\">" type={ name: string }
/// @definition.type symbol=NameOnly source="type NameOnly = Pick<Person, \"name\">" value=Pick<Person, "name">
/// @resolution.name source=Pick target=Pick
/// @resolution.name source=Person target=Person

const person: NameOnly = { name: "Ada", age: 42 };
/// @type.symbol symbol=person source=person type=NameOnly
/// @resolution.pattern source=person kind=binding target=person
/// @resolution.name source=NameOnly target=NameOnly
"#,
        r#"
/// @diagnostic.error id=excess-property message="unknown property 'age' in object literal for type 'NameOnly'"
/// @diagnostic.label line=9 column=26 span="{ name: \"Ada\", age: 42 }" line_source="const person: NameOnly = { name: \"Ada\", age: 42 };"
/// @diagnostic.related line=9 column=15 span="NameOnly" line_source="const person: NameOnly = { name: \"Ada\", age: 42 };" message="expected due to this annotation"
/// @diagnostic.note message="object literals may only specify known properties"
"#,
    );
}

#[test]
fn test_pick_preserves_readonly_field() {
    let session = TestSession::single(
        r#"
interface Person {
    readonly name: string;
    age: int32;
}

type NameOnly = Pick<Person, "name">;

const person: NameOnly = { name: "Ada" };
person.name = "Grace";
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Person {
    readonly name: string;
    age: int32;
}

type NameOnly = Pick<Person, "name">;

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

type NameOnly = Pick<Person, "name">;
/// @type.symbol symbol=NameOnly source="type NameOnly = Pick<Person, \"name\">" type={ readonly name: string }
/// @definition.type symbol=NameOnly source="type NameOnly = Pick<Person, \"name\">" value=Pick<Person, "name">
/// @resolution.name source=Pick target=Pick
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
