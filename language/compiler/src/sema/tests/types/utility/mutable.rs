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

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Person {
    readonly name: string;
    age: int32;
}

let person: { name: string; age: int32 } = { name: "Ada", age: 42 };

person.name = "Grace";
person.name satisfies string;

=== dir ===
interface Person {
/// @type.symbol symbol=Person type=Person
/// @definition.interface symbol=Person
/// @definition.where symbol=Person relation=satisfies left=this right=Person
/// @definition.field symbol=Person.age source="age: int32" key=age type=int32
/// @definition.field symbol=Person.name source="readonly name: string" key=name type=string

    readonly name: string;
    /// @type.symbol symbol=Person.name source="readonly name: string" type=string

    age: int32;
    /// @type.symbol symbol=Person.age source="age: int32" type=int32

}

let person: MutableFields<Person> = { name: "Ada", age: 42 };
/// @type.symbol symbol=person source=person type={ name: string; age: int32 }
/// @resolution.pattern source=person kind=binding target=person
/// @resolution.name source=MutableFields target=MutableFields
/// @resolution.name source=Person target=Person

person.name = "Grace";
/// @resolution.name source=person target=person
/// @resolution.place source=person placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=person root=person
/// @resolution.pattern.assign source=person.name kind=place
/// @resolution.access source=person.name root=person keys=[name]
/// @resolution.assignment source=person.name write="receiver={ name: string; age: int32 }, target=field(receiver={ name: string; age: int32 }, target=name, type=string), type=string" type=string

person.name satisfies string;
/// @resolution.name source=person target=person
/// @resolution.member source=person.name receiver={ name: string; age: int32 } type=string kind=field target_receiver={ name: string; age: int32 } key=name target_type=string
/// @resolution.place source=person placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=person root=person
/// @resolution.place source=person.name placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=person.name root=person keys=[name]
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

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Person {
    readonly name?: string;
}

const empty: { name?: string } = {};
const named: { name?: string } = { name: "Ada" };

empty satisfies MutableFields<Person>;
named satisfies MutableFields<Person>;

=== dir ===
interface Person {
/// @type.symbol symbol=Person type=Person
/// @definition.interface symbol=Person
/// @definition.where symbol=Person relation=satisfies left=this right=Person
/// @definition.field symbol=Person.name source="readonly name?: string" key=name type=string

    readonly name?: string;
    /// @type.symbol symbol=Person.name source="readonly name?: string" type=string

}

const empty: MutableFields<Person> = {};
/// @type.symbol symbol=empty source=empty type={ name?: string }
/// @resolution.pattern source=empty kind=binding target=empty
/// @resolution.name source=MutableFields target=MutableFields
/// @resolution.name source=Person target=Person

const named: MutableFields<Person> = { name: "Ada" };
/// @type.symbol symbol=named source=named type={ name?: string }
/// @resolution.pattern source=named kind=binding target=named
/// @resolution.name source=MutableFields target=MutableFields
/// @resolution.name source=Person target=Person

empty satisfies MutableFields<Person>;
/// @resolution.name source=empty target=empty
/// @resolution.place source=empty placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=empty root=empty
/// @resolution.name source=MutableFields target=MutableFields
/// @resolution.name source=Person target=Person

named satisfies MutableFields<Person>;
/// @resolution.name source=named target=named
/// @resolution.place source=named placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=named root=named
/// @resolution.name source=MutableFields target=MutableFields
/// @resolution.name source=Person target=Person
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

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Person {
    readonly profile: readonly {
        name: string;
    };
}

let person: { profile: readonly { name: string } } = { profile: { name: "Ada" } };

person.profile.name = "Grace";

=== dir ===
interface Person {
/// @type.symbol symbol=Person type=Person
/// @definition.interface symbol=Person
/// @definition.where symbol=Person relation=satisfies left=this right=Person
/// @definition.field symbol=Person.profile key=profile type=Readonly<{ name: string }>

    readonly profile: readonly {
    /// @type.symbol symbol=Person.profile type=Readonly<{ name: string }>

        name: string;
    };
}

let person: MutableFields<Person> = { profile: { name: "Ada" } };
/// @type.symbol symbol=person source=person type={ profile: Readonly<{ name: string }> }
/// @resolution.pattern source=person kind=binding target=person
/// @resolution.name source=MutableFields target=MutableFields
/// @resolution.name source=Person target=Person

person.profile.name = "Grace";
/// @resolution.name source=person target=person
/// @resolution.member source=person.profile receiver={ profile: Readonly<{ name: string }> } type=Readonly<{ name: string }> kind=field target_receiver={ profile: Readonly<{ name: string }> } key=profile target_type=Readonly<{ name: string }>
/// @resolution.place source=person placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=person root=person
/// @resolution.place source=person.profile placement="local" lifetime="static" access="readonly"
/// @resolution.access source=person.profile root=person keys=[profile]
/// @resolution.pattern.assign source=person.profile.name kind=place
/// @resolution.access source=person.profile.name root=person keys=[profile, name]
/// @resolution.assignment source=person.profile.name write="receiver=Readonly<{ name: string }>, target=field(receiver=Readonly<{ name: string }>, target=name, type=Readonly<string>), type=Readonly<string>" type=Readonly<string>
"#,
        r#"
/// @diagnostic.error id=cannot-assign-readonly-member message="cannot assign to readonly member 'name'"
/// @diagnostic.label line=10 column=16 span="name" line_source="person.profile.name = \"Grace\";"
"#,
    );
}
