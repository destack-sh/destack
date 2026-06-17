use crate::tests::{DirRows, TestSession};

#[test]
fn test_intersection_merges_object_shape_members() {
    let session = TestSession::single(
        r#"
type Named = { name: string };
type Aged = { age: int32 };
type Person = Named & Aged;

declare const person: Person;
const name = person.name;
const age = person.age;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Named = { name: string };
type Aged = { age: int32 };
type Person = Named & Aged;

declare const person: Person;
const name: string = person.name;
const age: int32 = person.age;

=== checked ===
type Named = { name: string };
/// @type.symbol symbol=Named source="type Named = { name: string }" type={ name: string }
/// @definition.type symbol=Named source="type Named = { name: string }" value={ name: string }

type Aged = { age: int32 };
/// @type.symbol symbol=Aged source="type Aged = { age: int32 }" type={ age: int32 }
/// @definition.type symbol=Aged source="type Aged = { age: int32 }" value={ age: int32 }

type Person = Named & Aged;
/// @type.symbol symbol=Person source="type Person = Named & Aged" type={ name: string; age: int32 }
/// @definition.type symbol=Person source="type Person = Named & Aged" value={ name: string; age: int32 }
/// @resolution.name source=Named target=Named
/// @resolution.name source=Aged target=Aged

declare const person: Person;
/// @type.symbol symbol=person source=person type={ name: string } & { age: int32 }
/// @resolution.name source=Person target=Person

const name = person.name;
/// @type.symbol symbol=name source=name type=string
/// @resolution.name source=person target=person
/// @resolution.member source=person.name receiver={ name: string } & { age: int32 } kind=field key=name

const age = person.age;
/// @type.symbol symbol=age source=age type=int32
/// @resolution.name source=person target=person
/// @resolution.member source=person.age receiver={ name: string } & { age: int32 } kind=field key=age
"#,
    );
}

#[test]
fn test_intersection_rejects_missing_field() {
    let session = TestSession::single(
        r#"
type Named = { name: string };
type Aged = { age: int32 };
type Person = Named & Aged;

const person: Person = { name: "Ada" };
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Named = { name: string };
type Aged = { age: int32 };
type Person = Named & Aged;

const person: Person = { name: "Ada" };

=== checked ===
type Named = { name: string };
/// @type.symbol symbol=Named source="type Named = { name: string }" type={ name: string }
/// @definition.type symbol=Named source="type Named = { name: string }" value={ name: string }

type Aged = { age: int32 };
/// @type.symbol symbol=Aged source="type Aged = { age: int32 }" type={ age: int32 }
/// @definition.type symbol=Aged source="type Aged = { age: int32 }" value={ age: int32 }

type Person = Named & Aged;
/// @type.symbol symbol=Person source="type Person = Named & Aged" type={ name: string; age: int32 }
/// @definition.type symbol=Person source="type Person = Named & Aged" value={ name: string; age: int32 }
/// @resolution.name source=Named target=Named
/// @resolution.name source=Aged target=Aged

const person: Person = { name: "Ada" };
/// @type.symbol symbol=person source=person type={ name: string } & { age: int32 }
/// @resolution.name source=Person target=Person
"#,
        r#"
/// @diagnostic.error code=EC215 message="missing required property 'age' for type 'Person'"
/// @diagnostic.label line=6 column=7 source="const person: Person = { name: \"Ada\" };"
"#,
    );
}

#[test]
fn test_intersection_rejects_incompatible_overlap() {
    let session = TestSession::single(
        r#"
type NumberValue = { value: int32 };
type TextValue = { value: string };
type Value = NumberValue & TextValue;

const value: Value = { value: "ok" };
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type NumberValue = { value: int32 };
type TextValue = { value: string };
type Value = NumberValue & TextValue;

const value: Value = { value: "ok" };

=== checked ===
type NumberValue = { value: int32 };
/// @type.symbol symbol=NumberValue source="type NumberValue = { value: int32 }" type={ value: int32 }
/// @definition.type symbol=NumberValue source="type NumberValue = { value: int32 }" value={ value: int32 }

type TextValue = { value: string };
/// @type.symbol symbol=TextValue source="type TextValue = { value: string }" type={ value: string }
/// @definition.type symbol=TextValue source="type TextValue = { value: string }" value={ value: string }

type Value = NumberValue & TextValue;
/// @type.symbol symbol=Value source="type Value = NumberValue & TextValue" type={ value: never }
/// @definition.type symbol=Value source="type Value = NumberValue & TextValue" value={ value: never }
/// @resolution.name source=NumberValue target=NumberValue
/// @resolution.name source=TextValue target=TextValue

const value: Value = { value: "ok" };
/// @type.symbol symbol=value source=value type={ value: int32 } & { value: string }
/// @resolution.name source=Value target=Value
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '{ value: \"ok\" }' is not assignable to type 'Value'"
/// @diagnostic.label line=6 column=7 source="const value: Value = { value: \"ok\" };"
"#,
    );
}

#[test]
fn test_intersection_preserves_compatible_overlap() {
    let session = TestSession::single(
        r#"
type Wide = { value: string | int32 };
type Narrow = { value: string; extra: string };
type Value = Wide & Narrow;

const value: Value = { value: "ok", extra: "yes" };
value.value satisfies string;
value.extra satisfies string;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Wide = { value: string | int32 };
type Narrow = { value: string; extra: string };
type Value = Wide & Narrow;

const value: Value = { value: "ok", extra: "yes" };
value.value satisfies string;
value.extra satisfies string;

=== checked ===
type Wide = { value: string | int32 };
/// @type.symbol symbol=Wide source="type Wide = { value: string | int32 }" type={ value: string | int32 }
/// @definition.type symbol=Wide source="type Wide = { value: string | int32 }" value={ value: string | int32 }

type Narrow = { value: string; extra: string };
/// @type.symbol symbol=Narrow source="type Narrow = { value: string; extra: string }" type={ value: string; extra: string }
/// @definition.type symbol=Narrow source="type Narrow = { value: string; extra: string }" value={ value: string; extra: string }

type Value = Wide & Narrow;
/// @type.symbol symbol=Value source="type Value = Wide & Narrow" type={ value: string; extra: string }
/// @definition.type symbol=Value source="type Value = Wide & Narrow" value={ value: string; extra: string }
/// @resolution.name source=Wide target=Wide
/// @resolution.name source=Narrow target=Narrow

const value: Value = { value: "ok", extra: "yes" };
/// @type.symbol symbol=value source=value type={ value: string | int32 } & { value: string; extra: string }
/// @resolution.name source=Value target=Value

value.value satisfies string;
/// @resolution.name source=value target=value
/// @resolution.member source=value.value receiver={ value: string | int32 } & { value: string; extra: string } kind=field key=value

value.extra satisfies string;
/// @resolution.name source=value target=value
/// @resolution.member source=value.extra receiver={ value: string | int32 } & { value: string; extra: string } kind=field key=extra
"#,
    );
}

#[test]
fn test_intersection_of_disjoint_primitives_yields_never() {
    let session = TestSession::single(
        r#"
type Both = string & int32;

let value: Both = "ok";
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Both = string & int32;

let value: Both = "ok";

=== checked ===
type Both = string & int32;
/// @type.symbol symbol=Both source="type Both = string & int32" type=never
/// @definition.type symbol=Both source="type Both = string & int32" value=never

let value: Both = "ok";
/// @type.symbol symbol=value source=value type=never
/// @resolution.name source=Both target=Both
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '\"ok\"' is not assignable to type 'Both'"
/// @diagnostic.label line=4 column=5 source="let value: Both = \"ok\";"
"#,
    );
}

#[test]
fn test_void_intersection_with_never_is_never() {
    let session = TestSession::single(
        r#"
type Value = void & never;

let value: Value = ();
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Value = void & never;

let value: Value = ();

=== checked ===
type Value = void & never;
/// @type.symbol symbol=Value source="type Value = void & never" type=never
/// @definition.type symbol=Value source="type Value = void & never" value=never

let value: Value = ();
/// @type.symbol symbol=value source=value type=never
/// @resolution.name source=Value target=Value
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '()' is not assignable to type 'Value'"
/// @diagnostic.label line=4 column=5 source="let value: Value = ();"
"#,
    );
}
