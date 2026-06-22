use crate::tests::{DirRows, TestSession};

#[test]
fn test_conditional_type_selects_each_branch() {
    let session = TestSession::single(
        r#"
type Select<T> = T extends string ? "yes" : "no";
type Text = Select<string>;
type Number = Select<int32>;

declare const text: Text;
declare const number: Number;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Select<T> = T extends string ? "yes" : "no";
type Text = Select<string>;
type Number = Select<int32>;

declare const text: Text;
declare const number: Number;

=== checked ===
type Select<T> = T extends string ? "yes" : "no";
/// @generic.template symbol=Select parameters=(T)
/// @type.symbol symbol=Select source="type Select<T> = T extends string ? \"yes\" : \"no\"" type=T extends string ? "yes" : "no"
/// @definition.type symbol=Select source="type Select<T> = T extends string ? \"yes\" : \"no\"" template=LocalGenericTemplateId(0) value=T extends string ? "yes" : "no"
/// @type.symbol symbol=Select.T source=T type=T
/// @resolution.name source=T target=Select.T

type Text = Select<string>;
/// @type.symbol symbol=Text source="type Text = Select<string>" type="yes"
/// @definition.type symbol=Text source="type Text = Select<string>" value="yes"
/// @resolution.name source=Select target=Select

type Number = Select<int32>;
/// @type.symbol symbol=Number source="type Number = Select<int32>" type="no"
/// @definition.type symbol=Number source="type Number = Select<int32>" value="no"
/// @resolution.name source=Select target=Select

declare const text: Text;
/// @type.symbol symbol=text source=text type="yes"
/// @resolution.name source=Text target=Text

declare const number: Number;
/// @type.symbol symbol=number source=number type="no"
/// @resolution.name source=Number target=Number
"#,
    );
}

#[test]
fn test_conditional_type_distributes_naked_parameter() {
    let session = TestSession::single(
        r#"
type OnlyStrings<T> = T extends string ? T : never;
type Result = OnlyStrings<string | int32>;

declare const value: Result;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type OnlyStrings<T> = T extends string ? T : never;
type Result = OnlyStrings<string | int32>;

declare const value: Result;

=== checked ===
type OnlyStrings<T> = T extends string ? T : never;
/// @generic.template symbol=OnlyStrings parameters=(T)
/// @type.symbol symbol=OnlyStrings source="type OnlyStrings<T> = T extends string ? T : never" type=T extends string ? T : never
/// @definition.type symbol=OnlyStrings source="type OnlyStrings<T> = T extends string ? T : never" template=LocalGenericTemplateId(0) value=T extends string ? T : never
/// @type.symbol symbol=OnlyStrings.T source=T type=T
/// @resolution.name source=T target=OnlyStrings.T
/// @resolution.name source=T target=OnlyStrings.T

type Result = OnlyStrings<string | int32>;
/// @type.symbol symbol=Result source="type Result = OnlyStrings<string | int32>" type=string
/// @definition.type symbol=Result source="type Result = OnlyStrings<string | int32>" value=string
/// @resolution.name source=OnlyStrings target=OnlyStrings

declare const value: Result;
/// @type.symbol symbol=value source=value type=string
/// @resolution.name source=Result target=Result
"#,
    );
}

#[test]
fn test_conditional_type_tuple_wrapping_disables_distribution() {
    let session = TestSession::single(
        r#"
type Wrapped<T> = (T,) extends (string,) ? "yes" : "no";
type Result = Wrapped<string | int32>;

declare const value: Result;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Wrapped<T> = (T,) extends (string,) ? "yes" : "no";
type Result = Wrapped<string | int32>;

declare const value: Result;

=== checked ===
type Wrapped<T> = (T,) extends (string,) ? "yes" : "no";
/// @generic.template symbol=Wrapped parameters=(T)
/// @type.symbol symbol=Wrapped source="type Wrapped<T> = (T,) extends (string,) ? \"yes\" : \"no\"" type=(T,) extends (string,) ? "yes" : "no"
/// @definition.type symbol=Wrapped source="type Wrapped<T> = (T,) extends (string,) ? \"yes\" : \"no\"" template=LocalGenericTemplateId(0) value=(T,) extends (string,) ? "yes" : "no"
/// @type.symbol symbol=Wrapped.T source=T type=T
/// @resolution.name source=T target=Wrapped.T

type Result = Wrapped<string | int32>;
/// @type.symbol symbol=Result source="type Result = Wrapped<string | int32>" type="no"
/// @definition.type symbol=Result source="type Result = Wrapped<string | int32>" value="no"
/// @resolution.name source=Wrapped target=Wrapped

declare const value: Result;
/// @type.symbol symbol=value source=value type="no"
/// @resolution.name source=Result target=Result
"#,
    );
}

#[test]
fn test_conditional_type_never_distribution_stays_never() {
    let session = TestSession::single(
        r#"
type OnlyStrings<T> = T extends string ? T : never;
type Result = OnlyStrings<never>;

let value: Result = "no";
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type OnlyStrings<T> = T extends string ? T : never;
type Result = OnlyStrings<never>;

let value: Result = "no";

=== checked ===
type OnlyStrings<T> = T extends string ? T : never;
/// @generic.template symbol=OnlyStrings parameters=(T)
/// @type.symbol symbol=OnlyStrings source="type OnlyStrings<T> = T extends string ? T : never" type=T extends string ? T : never
/// @definition.type symbol=OnlyStrings source="type OnlyStrings<T> = T extends string ? T : never" template=LocalGenericTemplateId(0) value=T extends string ? T : never
/// @type.symbol symbol=OnlyStrings.T source=T type=T
/// @resolution.name source=T target=OnlyStrings.T
/// @resolution.name source=T target=OnlyStrings.T

type Result = OnlyStrings<never>;
/// @type.symbol symbol=Result source="type Result = OnlyStrings<never>" type=never
/// @definition.type symbol=Result source="type Result = OnlyStrings<never>" value=never
/// @resolution.name source=OnlyStrings target=OnlyStrings

let value: Result = "no";
/// @type.symbol symbol=value source=value type=never
/// @resolution.name source=Result target=Result
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '\"no\"' is not assignable to type 'Result'"
/// @diagnostic.label line=5 column=5 source="let value: Result = \"no\";"
"#,
    );
}

#[test]
fn test_conditional_type_infer_extracts_object_member() {
    let session = TestSession::single(
        r#"
type Box<T> = { value: T };
type Unbox<T> = T extends Box<infer U> ? U : never;
type Value = Unbox<Box<"ready">>;

declare const value: Value;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Box<T> = { value: T };
type Unbox<T> = T extends Box<infer U> ? U : never;
type Value = Unbox<Box<"ready">>;

declare const value: Value;

=== checked ===
type Box<T> = { value: T };
/// @generic.template symbol=Box parameters=(T)
/// @type.symbol symbol=Box source="type Box<T> = { value: T }" type={ value: T }
/// @definition.type symbol=Box source="type Box<T> = { value: T }" template=LocalGenericTemplateId(0) value={ value: T }
/// @type.symbol symbol=Box.T source=T type=T
/// @resolution.name source=T target=Box.T

type Unbox<T> = T extends Box<infer U> ? U : never;
/// @generic.template symbol=Unbox parameters=(T)
/// @type.symbol symbol=Unbox source="type Unbox<T> = T extends Box<infer U> ? U : never" type=T extends Box<infer U> ? U : never
/// @definition.type symbol=Unbox source="type Unbox<T> = T extends Box<infer U> ? U : never" template=LocalGenericTemplateId(1) value=T extends Box<infer U> ? U : never
/// @type.symbol symbol=Unbox.T source=T type=T
/// @resolution.name source=T target=Unbox.T
/// @resolution.name source=Box target=Box
/// @generic.infer symbol=U constraint=unknown
/// @resolution.name source=U target=U

type Value = Unbox<Box<"ready">>;
/// @type.symbol symbol=Value source="type Value = Unbox<Box<\"ready\">>" type="ready"
/// @definition.type symbol=Value source="type Value = Unbox<Box<\"ready\">>" value="ready"
/// @resolution.name source=Unbox target=Unbox
/// @resolution.name source=Box target=Box

declare const value: Value;
/// @type.symbol symbol=value source=value type="ready"
/// @resolution.name source=Value target=Value
"#,
    );
}

#[test]
fn test_conditional_type_infer_can_ignore_unnamed_member() {
    let session = TestSession::single(
        r#"
type Box<T> = { value: T };
type IsBox<T> = T extends Box<infer _> ? true : false;
type Yes = IsBox<Box<string>>;
type No = IsBox<string>;

const yes: Yes = true;
const no: No = false;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Box<T> = { value: T };
type IsBox<T> = T extends Box<infer _> ? true : false;
type Yes = IsBox<Box<string>>;
type No = IsBox<string>;

const yes: Yes = true;
const no: No = false;

=== checked ===
type Box<T> = { value: T };
/// @generic.template symbol=Box parameters=(T)
/// @type.symbol symbol=Box source="type Box<T> = { value: T }" type={ value: T }
/// @definition.type symbol=Box source="type Box<T> = { value: T }" template=LocalGenericTemplateId(0) value={ value: T }
/// @type.symbol symbol=Box.T source=T type=T
/// @resolution.name source=T target=Box.T

type IsBox<T> = T extends Box<infer _> ? true : false;
/// @generic.template symbol=IsBox parameters=(T)
/// @type.symbol symbol=IsBox source="type IsBox<T> = T extends Box<infer _> ? true : false" type=T extends Box<infer _> ? true : false
/// @definition.type symbol=IsBox source="type IsBox<T> = T extends Box<infer _> ? true : false" template=LocalGenericTemplateId(1) value=T extends Box<infer _> ? true : false
/// @type.symbol symbol=IsBox.T source=T type=T
/// @resolution.name source=T target=IsBox.T
/// @resolution.name source=Box target=Box
/// @generic.infer symbol=_ constraint=unknown

type Yes = IsBox<Box<string>>;
/// @type.symbol symbol=Yes source="type Yes = IsBox<Box<string>>" type=true
/// @definition.type symbol=Yes source="type Yes = IsBox<Box<string>>" value=true
/// @resolution.name source=IsBox target=IsBox
/// @resolution.name source=Box target=Box

type No = IsBox<string>;
/// @type.symbol symbol=No source="type No = IsBox<string>" type=false
/// @definition.type symbol=No source="type No = IsBox<string>" value=false
/// @resolution.name source=IsBox target=IsBox

const yes: Yes = true;
/// @type.symbol symbol=yes source=yes type=true
/// @resolution.name source=Yes target=Yes

const no: No = false;
/// @type.symbol symbol=no source=no type=false
/// @resolution.name source=No target=No
"#,
    );
}

#[test]
fn test_conditional_type_infer_distributes_over_union() {
    let session = TestSession::single(
        r#"
type Box<T> = { value: T };
type Unbox<T> = T extends Box<infer U> ? U : never;
type Value = Unbox<Box<"a"> | Box<"b">>;

const first: Value = "a";
const second: Value = "b";
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Box<T> = { value: T };
type Unbox<T> = T extends Box<infer U> ? U : never;
type Value = Unbox<Box<"a"> | Box<"b">>;

const first: Value = "a" as Value;
const second: Value = "b" as Value;

=== checked ===
type Box<T> = { value: T };
/// @generic.template symbol=Box parameters=(T)
/// @type.symbol symbol=Box source="type Box<T> = { value: T }" type={ value: T }
/// @definition.type symbol=Box source="type Box<T> = { value: T }" template=LocalGenericTemplateId(0) value={ value: T }
/// @type.symbol symbol=Box.T source=T type=T
/// @resolution.name source=T target=Box.T

type Unbox<T> = T extends Box<infer U> ? U : never;
/// @generic.template symbol=Unbox parameters=(T)
/// @type.symbol symbol=Unbox source="type Unbox<T> = T extends Box<infer U> ? U : never" type=T extends Box<infer U> ? U : never
/// @definition.type symbol=Unbox source="type Unbox<T> = T extends Box<infer U> ? U : never" template=LocalGenericTemplateId(1) value=T extends Box<infer U> ? U : never
/// @type.symbol symbol=Unbox.T source=T type=T
/// @resolution.name source=T target=Unbox.T
/// @resolution.name source=Box target=Box
/// @generic.infer symbol=U constraint=unknown
/// @resolution.name source=U target=U

type Value = Unbox<Box<"a"> | Box<"b">>;
/// @type.symbol symbol=Value source="type Value = Unbox<Box<\"a\"> | Box<\"b\">>" type="a" | "b"
/// @definition.type symbol=Value source="type Value = Unbox<Box<\"a\"> | Box<\"b\">>" value="a" | "b"
/// @resolution.name source=Unbox target=Unbox
/// @resolution.name source=Box target=Box
/// @resolution.name source=Box target=Box

const first: Value = "a";
/// @type.symbol symbol=first source=first type="a" | "b"
/// @resolution.name source=Value target=Value

const second: Value = "b";
/// @type.symbol symbol=second source=second type="a" | "b"
/// @resolution.name source=Value target=Value
"#,
    );
}
