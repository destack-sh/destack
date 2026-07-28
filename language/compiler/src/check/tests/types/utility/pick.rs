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

declare const person: Pick<Person, "name" | "active">;

person.name satisfies string;
person.active satisfies boolean;

=== checked ===
interface Person {
/// @type.symbol symbol=Person type=Person
/// @definition.interface symbol=Person
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
/// @type.symbol symbol=person source=person type=Pick<Person, "name" | "active"> reduced={ name: string; active: boolean }
/// @resolution.pattern source=person kind=binding target=person
/// @resolution.name source=Pick target=types.object.Pick
/// @resolution.name source=Person target=Person

person.name satisfies string;
/// @resolution.name source=person target=person
/// @resolution.member source=person.name receiver={ name: string; active: boolean } type=string kind=field target_receiver={ name: string; active: boolean } key=name target_type=string
/// @resolution.place source=person placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=person root=person
/// @resolution.place source=person.name placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=person.name root=person keys=[name]

person.active satisfies boolean;
/// @resolution.name source=person target=person
/// @resolution.member source=person.active receiver={ name: string; active: boolean } type=boolean kind=field target_receiver={ name: string; active: boolean } key=active target_type=boolean
/// @resolution.place source=person placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=person root=person
/// @resolution.place source=person.active placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=person.active root=person keys=[active]

/// @generic.instance id="Pick<Person, \"name\" | \"active\">" template=types.object.Pick arguments=(Person, "name" | "active")
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

declare const person: Pick<Person, "name" | "active">;
const age = person.age;

=== checked ===
interface Person {
/// @type.symbol symbol=Person type=Person
/// @definition.interface symbol=Person
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
/// @type.symbol symbol=person source=person type=Pick<Person, "name" | "active"> reduced={ name: string; active: boolean }
/// @resolution.pattern source=person kind=binding target=person
/// @resolution.name source=Pick target=types.object.Pick
/// @resolution.name source=Person target=Person

const age = person.age;
/// @type.symbol symbol=age source=age type=<error>
/// @resolution.pattern source=age kind=binding target=age
/// @resolution.name source=person target=person
/// @resolution.place source=person placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=person root=person

/// @generic.instance id="Pick<Person, \"name\" | \"active\">" template=types.object.Pick arguments=(Person, "name" | "active")
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

    session.assert_dir_checked(
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
const aged: AgeOnly = { age: 42 };

empty satisfies AgeOnly;
aged satisfies AgeOnly;

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

type AgeOnly = Pick<Person, "age">;
/// @type.symbol symbol=AgeOnly source="type AgeOnly = Pick<Person, \"age\">" type=Pick<Person, "age"> reduced={ age?: int32 }
/// @definition.type symbol=AgeOnly source="type AgeOnly = Pick<Person, \"age\">" value=Pick<Person, "age"> reduced={ age?: int32 }
/// @resolution.name source=Pick target=types.object.Pick
/// @resolution.name source=Person target=Person

const empty: AgeOnly = {};
/// @type.symbol symbol=empty source=empty type=AgeOnly reduced={ age?: int32 }
/// @resolution.pattern source=empty kind=binding target=empty
/// @resolution.name source=AgeOnly target=AgeOnly

const aged: AgeOnly = { age: 42 };
/// @type.symbol symbol=aged source=aged type=AgeOnly reduced={ age?: int32 }
/// @resolution.pattern source=aged kind=binding target=aged
/// @resolution.name source=AgeOnly target=AgeOnly

empty satisfies AgeOnly;
/// @resolution.name source=empty target=empty
/// @resolution.place source=empty placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=empty root=empty
/// @resolution.name source=AgeOnly target=AgeOnly

aged satisfies AgeOnly;
/// @resolution.name source=aged target=aged
/// @resolution.place source=aged placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=aged root=aged
/// @resolution.name source=AgeOnly target=AgeOnly

/// @generic.instance id="Pick<Person, \"age\">" template=types.object.Pick arguments=(Person, "age")
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

    session.assert_dir_checked_and_diagnostics(
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

type AgeOnly = Pick<Person, "age">;
/// @type.symbol symbol=AgeOnly source="type AgeOnly = Pick<Person, \"age\">" type=Pick<Person, "age"> reduced={ age?: int32 }
/// @definition.type symbol=AgeOnly source="type AgeOnly = Pick<Person, \"age\">" value=Pick<Person, "age"> reduced={ age?: int32 }
/// @resolution.name source=Pick target=types.object.Pick
/// @resolution.name source=Person target=Person

const person: AgeOnly = { name: "Ada" };
/// @type.symbol symbol=person source=person type=AgeOnly reduced={ age?: int32 }
/// @resolution.pattern source=person kind=binding target=person
/// @resolution.name source=AgeOnly target=AgeOnly

/// @generic.instance id="Pick<Person, \"age\">" template=types.object.Pick arguments=(Person, "age")
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

    session.assert_dir_checked(
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

type NameOnly = Pick<Person, "name">;
/// @type.symbol symbol=NameOnly source="type NameOnly = Pick<Person, \"name\">" type=Pick<Person, "name"> reduced={ name: string }
/// @definition.type symbol=NameOnly source="type NameOnly = Pick<Person, \"name\">" value=Pick<Person, "name"> reduced={ name: string }
/// @resolution.name source=Pick target=types.object.Pick
/// @resolution.name source=Person target=Person

const person: NameOnly = { name: "Ada" };
/// @type.symbol symbol=person source=person type=NameOnly reduced={ name: string }
/// @resolution.pattern source=person kind=binding target=person
/// @resolution.name source=NameOnly target=NameOnly

person satisfies NameOnly;
/// @resolution.name source=person target=person
/// @resolution.place source=person placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=person root=person
/// @resolution.name source=NameOnly target=NameOnly

/// @generic.instance id="Pick<Person, \"name\">" template=types.object.Pick arguments=(Person, "name")
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

    session.assert_dir_checked_and_diagnostics(
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

type NameOnly = Pick<Person, "name">;
/// @type.symbol symbol=NameOnly source="type NameOnly = Pick<Person, \"name\">" type=Pick<Person, "name"> reduced={ name: string }
/// @definition.type symbol=NameOnly source="type NameOnly = Pick<Person, \"name\">" value=Pick<Person, "name"> reduced={ name: string }
/// @resolution.name source=Pick target=types.object.Pick
/// @resolution.name source=Person target=Person

const person: NameOnly = {};
/// @type.symbol symbol=person source=person type=NameOnly reduced={ name: string }
/// @resolution.pattern source=person kind=binding target=person
/// @resolution.name source=NameOnly target=NameOnly

/// @generic.instance id="Pick<Person, \"name\">" template=types.object.Pick arguments=(Person, "name")
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

    session.assert_dir_checked_and_diagnostics(
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

type NameOnly = Pick<Person, "name">;
/// @type.symbol symbol=NameOnly source="type NameOnly = Pick<Person, \"name\">" type=Pick<Person, "name"> reduced={ name: string }
/// @definition.type symbol=NameOnly source="type NameOnly = Pick<Person, \"name\">" value=Pick<Person, "name"> reduced={ name: string }
/// @resolution.name source=Pick target=types.object.Pick
/// @resolution.name source=Person target=Person

const person: NameOnly = { name: "Ada", age: 42 };
/// @type.symbol symbol=person source=person type=NameOnly reduced={ name: string }
/// @resolution.pattern source=person kind=binding target=person
/// @resolution.name source=NameOnly target=NameOnly

/// @generic.instance id="Pick<Person, \"name\">" template=types.object.Pick arguments=(Person, "name")
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

    session.assert_dir_checked_and_diagnostics(
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

type NameOnly = Pick<Person, "name">;
/// @type.symbol symbol=NameOnly source="type NameOnly = Pick<Person, \"name\">" type=Pick<Person, "name"> reduced={ readonly name: string }
/// @definition.type symbol=NameOnly source="type NameOnly = Pick<Person, \"name\">" value=Pick<Person, "name"> reduced={ readonly name: string }
/// @resolution.name source=Pick target=types.object.Pick
/// @resolution.name source=Person target=Person

const person: NameOnly = { name: "Ada" };
/// @type.symbol symbol=person source=person type=NameOnly reduced={ readonly name: string }
/// @resolution.pattern source=person kind=binding target=person
/// @resolution.name source=NameOnly target=NameOnly

person.name = "Grace";
/// @resolution.name source=person target=person
/// @resolution.place source=person placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=person root=person
/// @resolution.pattern.assign source=person.name kind=place
/// @resolution.assignment source=person.name write="receiver={ readonly name: string }, target=field(receiver={ readonly name: string }, target=name, type=string), type=string" type=string

/// @generic.instance id="Pick<Person, \"name\">" template=types.object.Pick arguments=(Person, "name")
"#,
        r#"
/// @diagnostic.error id=cannot-assign-readonly-member message="cannot assign to readonly member 'name'"
/// @diagnostic.label line=10 column=8 span="name" line_source="person.name = \"Grace\";"
"#,
    );
}
