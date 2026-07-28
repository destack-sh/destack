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
/// @type.symbol symbol=Person source="type Person = Named & Aged" type=Named & Aged reduced={ name: string; age: int32 }
/// @definition.type symbol=Person source="type Person = Named & Aged" value=Named & Aged reduced={ name: string; age: int32 }
/// @resolution.name source=Named target=Named
/// @resolution.name source=Aged target=Aged

declare const person: Person;
/// @type.symbol symbol=person source=person type=Person reduced={ name: string; age: int32 }
/// @resolution.pattern source=person kind=binding target=person
/// @resolution.name source=Person target=Person

const name = person.name;
/// @type.symbol symbol=name source=name type=string
/// @resolution.pattern source=name kind=binding target=name
/// @resolution.name source=person target=person
/// @resolution.member source=person.name receiver={ name: string; age: int32 } type=string kind=field target_receiver={ name: string; age: int32 } key=name target_type=string
/// @resolution.place source=person placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=person root=person
/// @resolution.access source=person.name root=person keys=[name]

const age = person.age;
/// @type.symbol symbol=age source=age type=int32
/// @resolution.pattern source=age kind=binding target=age
/// @resolution.name source=person target=person
/// @resolution.member source=person.age receiver={ name: string; age: int32 } type=int32 kind=field target_receiver={ name: string; age: int32 } key=age target_type=int32
/// @resolution.place source=person placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=person root=person
/// @resolution.access source=person.age root=person keys=[age]
"#,
    );
}

#[test]
fn test_intersection_member_preserves_each_interface_requirement() {
    let session = TestSession::single(
        r#"
interface Left {
    value: string;
}

interface Right {
    value: string;
}

declare const both: Left & Right;
const value = both.value;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Left {
    value: string;
}

interface Right {
    value: string;
}

declare const both: Left & Right;
const value: string = both.value;

=== checked ===
interface Left {
/// @type.symbol symbol=Left type=Left
/// @definition.interface symbol=Left
/// @definition.field symbol=Left.value source="value: string" key=value type=string

    value: string;
    /// @type.symbol symbol=Left.value source="value: string" type=string

}

interface Right {
/// @type.symbol symbol=Right type=Right
/// @definition.interface symbol=Right
/// @definition.field symbol=Right.value source="value: string" key=value type=string

    value: string;
    /// @type.symbol symbol=Right.value source="value: string" type=string

}

declare const both: Left & Right;
/// @type.symbol symbol=both source=both type=Left & Right
/// @resolution.pattern source=both kind=binding target=both
/// @resolution.name source=Left target=Left
/// @resolution.name source=Right target=Right

const value = both.value;
/// @type.symbol symbol=value source=value type=string
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=both target=both
/// @resolution.member source=both.value receiver=Left & Right type=string kind=intersection targets=[field(receiver=Left & Right, target=Left.value, type=string), field(receiver=Left & Right, target=Right.value, type=string)]
/// @resolution.place source=both placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=both root=both
/// @resolution.access source=both.value root=both keys=[value]
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
/// @type.symbol symbol=Person source="type Person = Named & Aged" type=Named & Aged reduced={ name: string; age: int32 }
/// @definition.type symbol=Person source="type Person = Named & Aged" value=Named & Aged reduced={ name: string; age: int32 }
/// @resolution.name source=Named target=Named
/// @resolution.name source=Aged target=Aged

const person: Person = { name: "Ada" };
/// @type.symbol symbol=person source=person type=Person reduced={ name: string; age: int32 }
/// @resolution.pattern source=person kind=binding target=person
/// @resolution.name source=Person target=Person
"#,
        r#"
/// @diagnostic.error id=missing-required-property message="missing required property 'age' for type 'Person'"
/// @diagnostic.label line=6 column=24 span="{ name: \"Ada\" }" line_source="const person: Person = { name: \"Ada\" };"
/// @diagnostic.related line=6 column=15 span="Person" line_source="const person: Person = { name: \"Ada\" };" message="expected due to this annotation"
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
/// @type.symbol symbol=Value source="type Value = NumberValue & TextValue" type=NumberValue & TextValue reduced={ value: int32 & string }
/// @definition.type symbol=Value source="type Value = NumberValue & TextValue" value=NumberValue & TextValue reduced={ value: int32 & string }
/// @resolution.name source=NumberValue target=NumberValue
/// @resolution.name source=TextValue target=TextValue

const value: Value = { value: "ok" };
/// @type.symbol symbol=value source=value type=Value reduced={ value: int32 & string }
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Value target=Value
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"ok\"' is not assignable to type 'int32 & string'"
/// @diagnostic.label line=6 column=31 span="\"ok\"" line_source="const value: Value = { value: \"ok\" };"
/// @diagnostic.related line=6 column=14 span="Value" line_source="const value: Value = { value: \"ok\" };" message="expected due to this annotation"
/// @diagnostic.note message="the mismatch is in field 'value'"
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
/// @type.symbol symbol=Value source="type Value = Wide & Narrow" type=Wide & Narrow reduced={ value: string | int32 & string; extra: string }
/// @definition.type symbol=Value source="type Value = Wide & Narrow" value=Wide & Narrow reduced={ value: string | int32 & string; extra: string }
/// @resolution.name source=Wide target=Wide
/// @resolution.name source=Narrow target=Narrow

const value: Value = { value: "ok", extra: "yes" };
/// @type.symbol symbol=value source=value type=Value reduced={ value: string | int32 & string; extra: string }
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Value target=Value

value.value satisfies string;
/// @resolution.name source=value target=value
/// @resolution.member source=value.value receiver={ value: string | int32 & string; extra: string } type=string | int32 & string kind=field target_receiver={ value: string | int32 & string; extra: string } key=value target_type=string | int32 & string
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=value root=value
/// @resolution.place source=value.value placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=value.value root=value keys=[value]

value.extra satisfies string;
/// @resolution.name source=value target=value
/// @resolution.member source=value.extra receiver={ value: string | int32 & string; extra: string } type=string kind=field target_receiver={ value: string | int32 & string; extra: string } key=extra target_type=string
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=value root=value
/// @resolution.place source=value.extra placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=value.extra root=value keys=[extra]
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
/// @type.symbol symbol=Both source="type Both = string & int32" type=string & int32
/// @definition.type symbol=Both source="type Both = string & int32" value=string & int32

let value: Both = "ok";
/// @type.symbol symbol=value source=value type=Both reduced=string & int32
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Both target=Both
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"ok\"' is not assignable to type 'Both'"
/// @diagnostic.label line=4 column=19 span="\"ok\"" line_source="let value: Both = \"ok\";"
/// @diagnostic.related line=4 column=12 span="Both" line_source="let value: Both = \"ok\";" message="expected due to this annotation"
/// @diagnostic.note message="'Both' reduces to 'string & int32'"
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
/// @type.symbol symbol=Value source="type Value = void & never" type=void & never
/// @definition.type symbol=Value source="type Value = void & never" value=void & never

let value: Value = ();
/// @type.symbol symbol=value source=value type=Value reduced=void & never
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Value target=Value
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '()' is not assignable to type 'Value'"
/// @diagnostic.label line=4 column=20 span="()" line_source="let value: Value = ();"
/// @diagnostic.related line=4 column=12 span="Value" line_source="let value: Value = ();" message="expected due to this annotation"
/// @diagnostic.note message="'Value' reduces to 'void & never'"
"#,
    );
}
