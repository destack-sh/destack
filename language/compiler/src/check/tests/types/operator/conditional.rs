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
/// @definition.type symbol=Select source="type Select<T> = T extends string ? \"yes\" : \"no\"" template=(T) value=T extends string ? "yes" : "no"
/// @type.symbol symbol=Select.T source=T type=T
/// @resolution.name source=T target=Select.T

type Text = Select<string>;
/// @type.symbol symbol=Text source="type Text = Select<string>" type=Select<string> reduced="yes"
/// @definition.type symbol=Text source="type Text = Select<string>" value=Select<string> reduced="yes"
/// @resolution.name source=Select target=Select

type Number = Select<int32>;
/// @type.symbol symbol=Number source="type Number = Select<int32>" type=Select<int32> reduced="no"
/// @definition.type symbol=Number source="type Number = Select<int32>" value=Select<int32> reduced="no"
/// @resolution.name source=Select target=Select

declare const text: Text;
/// @type.symbol symbol=text source=text type=Text reduced="yes"
/// @resolution.pattern source=text kind=binding target=text
/// @resolution.name source=Text target=Text

declare const number: Number;
/// @type.symbol symbol=number source=number type=Number reduced="no"
/// @resolution.pattern source=number kind=binding target=number
/// @resolution.name source=Number target=Number

/// @generic.instance id=Select<int32> template=Select arguments=(int32)
/// @generic.instance id=Select<string> template=Select arguments=(string)
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
/// @definition.type symbol=OnlyStrings source="type OnlyStrings<T> = T extends string ? T : never" template=(T) value=T extends string ? T : never
/// @type.symbol symbol=OnlyStrings.T source=T type=T
/// @resolution.name source=T target=OnlyStrings.T
/// @resolution.name source=T target=OnlyStrings.T

type Result = OnlyStrings<string | int32>;
/// @type.symbol symbol=Result source="type Result = OnlyStrings<string | int32>" type=OnlyStrings<string | int32> reduced=string
/// @definition.type symbol=Result source="type Result = OnlyStrings<string | int32>" value=OnlyStrings<string | int32> reduced=string
/// @resolution.name source=OnlyStrings target=OnlyStrings

declare const value: Result;
/// @type.symbol symbol=value source=value type=Result reduced=string
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Result target=Result

/// @generic.instance id="OnlyStrings<string | int32>" template=OnlyStrings arguments=(string | int32)
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
/// @type.symbol symbol=Wrapped source="type Wrapped<T> = (T,) extends (string,) ? \"yes\" : \"no\"" type=(T,) extends (string,) ? "yes" : "no" reduced="no"
/// @definition.type symbol=Wrapped source="type Wrapped<T> = (T,) extends (string,) ? \"yes\" : \"no\"" template=(T) value=(T,) extends (string,) ? "yes" : "no" reduced="no"
/// @type.symbol symbol=Wrapped.T source=T type=T
/// @resolution.name source=T target=Wrapped.T

type Result = Wrapped<string | int32>;
/// @type.symbol symbol=Result source="type Result = Wrapped<string | int32>" type=Wrapped<string | int32> reduced="no"
/// @definition.type symbol=Result source="type Result = Wrapped<string | int32>" value=Wrapped<string | int32> reduced="no"
/// @resolution.name source=Wrapped target=Wrapped

declare const value: Result;
/// @type.symbol symbol=value source=value type=Result reduced="no"
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Result target=Result

/// @generic.instance id="Wrapped<string | int32>" template=Wrapped arguments=(string | int32)
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
/// @definition.type symbol=OnlyStrings source="type OnlyStrings<T> = T extends string ? T : never" template=(T) value=T extends string ? T : never
/// @type.symbol symbol=OnlyStrings.T source=T type=T
/// @resolution.name source=T target=OnlyStrings.T
/// @resolution.name source=T target=OnlyStrings.T

type Result = OnlyStrings<never>;
/// @type.symbol symbol=Result source="type Result = OnlyStrings<never>" type=OnlyStrings<never> reduced=never
/// @definition.type symbol=Result source="type Result = OnlyStrings<never>" value=OnlyStrings<never> reduced=never
/// @resolution.name source=OnlyStrings target=OnlyStrings

let value: Result = "no";
/// @type.symbol symbol=value source=value type=Result reduced=never
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Result target=Result

/// @generic.instance id=OnlyStrings<never> template=OnlyStrings arguments=(never)
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"no\"' is not assignable to type 'Result'"
/// @diagnostic.label line=5 column=21 span="\"no\"" line_source="let value: Result = \"no\";"
/// @diagnostic.related line=5 column=12 span="Result" line_source="let value: Result = \"no\";" message="expected due to this annotation"
/// @diagnostic.note message="'Result' reduces to 'never'"
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
/// @generic.template symbol=Box parameters=(T#1)
/// @type.symbol symbol=Box source="type Box<T> = { value: T }" type={ value: T#1 }
/// @definition.type symbol=Box source="type Box<T> = { value: T }" template=(T#1) value={ value: T#1 }
/// @type.symbol symbol=Box.T source=T type=T#1
/// @resolution.name source=T target=Box.T

type Unbox<T> = T extends Box<infer U> ? U : never;
/// @generic.template symbol=Unbox parameters=(T#2)
/// @type.symbol symbol=Unbox source="type Unbox<T> = T extends Box<infer U> ? U : never" type=T#2 extends Box<infer U> ? Unbox.U : never
/// @definition.type symbol=Unbox source="type Unbox<T> = T extends Box<infer U> ? U : never" template=(T#2) value=T#2 extends Box<infer U> ? Unbox.U : never
/// @type.symbol symbol=Unbox.T source=T type=T#2
/// @resolution.name source=T target=Unbox.T
/// @resolution.name source=Box target=Box
/// @resolution.name source=U target=Unbox.U

type Value = Unbox<Box<"ready">>;
/// @type.symbol symbol=Value source="type Value = Unbox<Box<\"ready\">>" type=Unbox<Box<"ready">> reduced="ready"
/// @definition.type symbol=Value source="type Value = Unbox<Box<\"ready\">>" value=Unbox<Box<"ready">> reduced="ready"
/// @resolution.name source=Unbox target=Unbox
/// @resolution.name source=Box target=Box

declare const value: Value;
/// @type.symbol symbol=value source=value type=Value reduced="ready"
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Value target=Value

/// @generic.instance id="Box<\"ready\">" template=Box arguments=("ready")
/// @generic.instance id="Box<infer U>" template=Box arguments=(infer U)
/// @generic.instance id="Unbox<Box<\"ready\">>" template=Unbox arguments=(Box<"ready">)
"#,
    );
}

#[test]
fn test_conditional_type_infer_inherits_constructor_parameter_bound() {
    let session = TestSession::single(
        r#"
newtype Vector<T, comptime N: int> = intrinsic;
type LaneCount<V> = V extends Vector<infer T, infer N> ? N : never;
type Count = LaneCount<Vector<string, 4>>;

declare const count: Count;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
newtype Vector<in out T, comptime N: int> = intrinsic;
type LaneCount<V> = V extends Vector<infer T, infer N> ? N : never;
type Count = LaneCount<Vector<string, 4>>;

declare const count: Count;

=== checked ===
newtype Vector<T, comptime N: int> = intrinsic;
/// @generic.template symbol=Vector parameters=(in out T#1, comptime N#1: int64)
/// @type.symbol symbol=Vector source="newtype Vector<T, comptime N: int> = intrinsic" type=Vector
/// @definition.newtype symbol=Vector source="newtype Vector<T, comptime N: int> = intrinsic" template=(in out T#1, comptime N#1: int64) backing=intrinsic
/// @type.symbol symbol=Vector.T source=T type=T#1
/// @type.symbol symbol=Vector.N source="comptime N: int" type=N#1

type LaneCount<V> = V extends Vector<infer T, infer N> ? N : never;
/// @generic.template symbol=LaneCount parameters=(V)
/// @type.symbol symbol=LaneCount source="type LaneCount<V> = V extends Vector<infer T, infer N> ? N : never" type=V extends Vector<infer T, infer N> ? LaneCount.N : never
/// @definition.type symbol=LaneCount source="type LaneCount<V> = V extends Vector<infer T, infer N> ? N : never" template=(V) value=V extends Vector<infer T, infer N> ? LaneCount.N : never
/// @type.symbol symbol=LaneCount.V source=V type=V
/// @resolution.name source=V target=LaneCount.V
/// @resolution.name source=Vector target=Vector
/// @resolution.name source=N target=LaneCount.N

type Count = LaneCount<Vector<string, 4>>;
/// @type.symbol symbol=Count source="type Count = LaneCount<Vector<string, 4>>" type=LaneCount<Vector<string, 4>> reduced=4
/// @definition.type symbol=Count source="type Count = LaneCount<Vector<string, 4>>" value=LaneCount<Vector<string, 4>> reduced=4
/// @resolution.name source=LaneCount target=LaneCount
/// @resolution.name source=Vector target=Vector

declare const count: Count;
/// @type.symbol symbol=count source=count type=Count reduced=4
/// @resolution.pattern source=count kind=binding target=count
/// @resolution.name source=Count target=Count

/// @generic.instance id="LaneCount<Vector<string, 4>>" template=LaneCount arguments=(Vector<string, 4>)
/// @generic.instance id="Vector<infer T, infer N>" template=Vector arguments=(infer T, infer N)
/// @generic.instance id="Vector<string, 4>" template=Vector arguments=(string, 4)
"#,
    );
}

#[test]
fn test_conditional_type_infer_constraint_rejects_non_matching_capture() {
    let session = TestSession::single(
        r#"
type Box<T> = { value: T };
type Text<T> = T extends Box<infer U extends string> ? U : never;
type Value = Text<Box<int32>>;

let value: Value = "no";
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Box<T> = { value: T };
type Text<T> = T extends Box<infer U extends string> ? U : never;
type Value = Text<Box<int32>>;

let value: Value = "no";

=== checked ===
type Box<T> = { value: T };
/// @generic.template symbol=Box parameters=(T#1)
/// @type.symbol symbol=Box source="type Box<T> = { value: T }" type={ value: T#1 }
/// @definition.type symbol=Box source="type Box<T> = { value: T }" template=(T#1) value={ value: T#1 }
/// @type.symbol symbol=Box.T source=T type=T#1
/// @resolution.name source=T target=Box.T

type Text<T> = T extends Box<infer U extends string> ? U : never;
/// @generic.template symbol=Text parameters=(T#2)
/// @type.symbol symbol=Text source="type Text<T> = T extends Box<infer U extends string> ? U : never" type=T#2 extends Box<infer U extends string> ? Text.U : never
/// @definition.type symbol=Text source="type Text<T> = T extends Box<infer U extends string> ? U : never" template=(T#2) value=T#2 extends Box<infer U extends string> ? Text.U : never
/// @type.symbol symbol=Text.T source=T type=T#2
/// @resolution.name source=T target=Text.T
/// @resolution.name source=Box target=Box
/// @resolution.name source=U target=Text.U

type Value = Text<Box<int32>>;
/// @type.symbol symbol=Value source="type Value = Text<Box<int32>>" type=Text<Box<int32>> reduced=never
/// @definition.type symbol=Value source="type Value = Text<Box<int32>>" value=Text<Box<int32>> reduced=never
/// @resolution.name source=Text target=Text
/// @resolution.name source=Box target=Box

let value: Value = "no";
/// @type.symbol symbol=value source=value type=Value reduced=never
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Value target=Value

/// @generic.instance id="Box<infer U extends string>" template=Box arguments=(infer U extends string)
/// @generic.instance id=Box<int32> template=Box arguments=(int32)
/// @generic.instance id=Text<Box<int32>> template=Text arguments=(Box<int32>)
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"no\"' is not assignable to type 'Value'"
/// @diagnostic.label line=6 column=20 span="\"no\"" line_source="let value: Value = \"no\";"
/// @diagnostic.related line=6 column=12 span="Value" line_source="let value: Value = \"no\";" message="expected due to this annotation"
/// @diagnostic.note message="'Value' reduces to 'never'"
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
/// @generic.template symbol=Box parameters=(T#1)
/// @type.symbol symbol=Box source="type Box<T> = { value: T }" type={ value: T#1 }
/// @definition.type symbol=Box source="type Box<T> = { value: T }" template=(T#1) value={ value: T#1 }
/// @type.symbol symbol=Box.T source=T type=T#1
/// @resolution.name source=T target=Box.T

type IsBox<T> = T extends Box<infer _> ? true : false;
/// @generic.template symbol=IsBox parameters=(T#2)
/// @type.symbol symbol=IsBox source="type IsBox<T> = T extends Box<infer _> ? true : false" type=T#2 extends Box<infer _> ? true : false
/// @definition.type symbol=IsBox source="type IsBox<T> = T extends Box<infer _> ? true : false" template=(T#2) value=T#2 extends Box<infer _> ? true : false
/// @type.symbol symbol=IsBox.T source=T type=T#2
/// @resolution.name source=T target=IsBox.T
/// @resolution.name source=Box target=Box

type Yes = IsBox<Box<string>>;
/// @type.symbol symbol=Yes source="type Yes = IsBox<Box<string>>" type=IsBox<Box<string>> reduced=true
/// @definition.type symbol=Yes source="type Yes = IsBox<Box<string>>" value=IsBox<Box<string>> reduced=true
/// @resolution.name source=IsBox target=IsBox
/// @resolution.name source=Box target=Box

type No = IsBox<string>;
/// @type.symbol symbol=No source="type No = IsBox<string>" type=IsBox<string> reduced=false
/// @definition.type symbol=No source="type No = IsBox<string>" value=IsBox<string> reduced=false
/// @resolution.name source=IsBox target=IsBox

const yes: Yes = true;
/// @type.symbol symbol=yes source=yes type=Yes reduced=true
/// @resolution.pattern source=yes kind=binding target=yes
/// @resolution.name source=Yes target=Yes

const no: No = false;
/// @type.symbol symbol=no source=no type=No reduced=false
/// @resolution.pattern source=no kind=binding target=no
/// @resolution.name source=No target=No

/// @generic.instance id="Box<infer _>" template=Box arguments=(infer _)
/// @generic.instance id=Box<string> template=Box arguments=(string)
/// @generic.instance id=IsBox<Box<string>> template=IsBox arguments=(Box<string>)
/// @generic.instance id=IsBox<string> template=IsBox arguments=(string)
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
/// @generic.template symbol=Box parameters=(T#1)
/// @type.symbol symbol=Box source="type Box<T> = { value: T }" type={ value: T#1 }
/// @definition.type symbol=Box source="type Box<T> = { value: T }" template=(T#1) value={ value: T#1 }
/// @type.symbol symbol=Box.T source=T type=T#1
/// @resolution.name source=T target=Box.T

type Unbox<T> = T extends Box<infer U> ? U : never;
/// @generic.template symbol=Unbox parameters=(T#2)
/// @type.symbol symbol=Unbox source="type Unbox<T> = T extends Box<infer U> ? U : never" type=T#2 extends Box<infer U> ? Unbox.U : never
/// @definition.type symbol=Unbox source="type Unbox<T> = T extends Box<infer U> ? U : never" template=(T#2) value=T#2 extends Box<infer U> ? Unbox.U : never
/// @type.symbol symbol=Unbox.T source=T type=T#2
/// @resolution.name source=T target=Unbox.T
/// @resolution.name source=Box target=Box
/// @resolution.name source=U target=Unbox.U

type Value = Unbox<Box<"a"> | Box<"b">>;
/// @type.symbol symbol=Value source="type Value = Unbox<Box<\"a\"> | Box<\"b\">>" type=Unbox<Box<"a"> | Box<"b">> reduced="a" | "b"
/// @definition.type symbol=Value source="type Value = Unbox<Box<\"a\"> | Box<\"b\">>" value=Unbox<Box<"a"> | Box<"b">> reduced="a" | "b"
/// @resolution.name source=Unbox target=Unbox
/// @resolution.name source=Box target=Box
/// @resolution.name source=Box target=Box

const first: Value = "a";
/// @type.symbol symbol=first source=first type=Value reduced="a" | "b"
/// @resolution.pattern source=first kind=binding target=first
/// @resolution.name source=Value target=Value

const second: Value = "b";
/// @type.symbol symbol=second source=second type=Value reduced="a" | "b"
/// @resolution.pattern source=second kind=binding target=second
/// @resolution.name source=Value target=Value

/// @generic.instance id="Box<\"a\">" template=Box arguments=("a")
/// @generic.instance id="Box<\"b\">" template=Box arguments=("b")
/// @generic.instance id="Box<infer U>" template=Box arguments=(infer U)
/// @generic.instance id="Unbox<Box<\"a\"> | Box<\"b\">>" template=Unbox arguments=(Box<"a"> | Box<"b">)
"#,
    );
}
