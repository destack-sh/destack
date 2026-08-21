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

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Select<T> = T extends string ? "yes" : "no";
type Text = Select<string>;
type Number = Select<int32>;

declare const text: "yes";
declare const number: "no";

=== dir ===
type Select<T> = T extends string ? "yes" : "no";
/// @generic.template symbol=Select parameters=(T)
/// @type.symbol symbol=Select source="type Select<T> = T extends string ? \"yes\" : \"no\"" type=T extends string ? "yes" : "no"
/// @definition.type symbol=Select source="type Select<T> = T extends string ? \"yes\" : \"no\"" template=(T) value=T extends string ? "yes" : "no"
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
/// @resolution.pattern source=text kind=binding target=text
/// @resolution.name source=Text target=Text

declare const number: Number;
/// @type.symbol symbol=number source=number type="no"
/// @resolution.pattern source=number kind=binding target=number
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

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type OnlyStrings<T> = T extends string ? T : never;
type Result = OnlyStrings<string | int32>;

declare const value: string;

=== dir ===
type OnlyStrings<T> = T extends string ? T : never;
/// @generic.template symbol=OnlyStrings parameters=(T)
/// @type.symbol symbol=OnlyStrings source="type OnlyStrings<T> = T extends string ? T : never" type=T extends string ? T : never
/// @definition.type symbol=OnlyStrings source="type OnlyStrings<T> = T extends string ? T : never" template=(T) value=T extends string ? T : never
/// @type.symbol symbol=OnlyStrings.T source=T type=T
/// @resolution.name source=T target=OnlyStrings.T
/// @resolution.name source=T target=OnlyStrings.T

type Result = OnlyStrings<string | int32>;
/// @type.symbol symbol=Result source="type Result = OnlyStrings<string | int32>" type=string
/// @definition.type symbol=Result source="type Result = OnlyStrings<string | int32>" value=string
/// @resolution.name source=OnlyStrings target=OnlyStrings

declare const value: Result;
/// @type.symbol symbol=value source=value type=string
/// @resolution.pattern source=value kind=binding target=value
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

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Wrapped<T> = (T,) extends (string,) ? "yes" : "no";
type Result = Wrapped<string | int32>;

declare const value: "no";

=== dir ===
type Wrapped<T> = (T,) extends (string,) ? "yes" : "no";
/// @generic.template symbol=Wrapped parameters=(T)
/// @type.symbol symbol=Wrapped source="type Wrapped<T> = (T,) extends (string,) ? \"yes\" : \"no\"" type=(T,) extends (string,) ? "yes" : "no"
/// @definition.type symbol=Wrapped source="type Wrapped<T> = (T,) extends (string,) ? \"yes\" : \"no\"" template=(T) value=(T,) extends (string,) ? "yes" : "no"
/// @type.symbol symbol=Wrapped.T source=T type=T
/// @resolution.name source=T target=Wrapped.T

type Result = Wrapped<string | int32>;
/// @type.symbol symbol=Result source="type Result = Wrapped<string | int32>" type="no"
/// @definition.type symbol=Result source="type Result = Wrapped<string | int32>" value="no"
/// @resolution.name source=Wrapped target=Wrapped

declare const value: Result;
/// @type.symbol symbol=value source=value type="no"
/// @resolution.pattern source=value kind=binding target=value
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

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type OnlyStrings<T> = T extends string ? T : never;
type Result = OnlyStrings<never>;

let value: never = "no";

=== dir ===
type OnlyStrings<T> = T extends string ? T : never;
/// @generic.template symbol=OnlyStrings parameters=(T)
/// @type.symbol symbol=OnlyStrings source="type OnlyStrings<T> = T extends string ? T : never" type=T extends string ? T : never
/// @definition.type symbol=OnlyStrings source="type OnlyStrings<T> = T extends string ? T : never" template=(T) value=T extends string ? T : never
/// @type.symbol symbol=OnlyStrings.T source=T type=T
/// @resolution.name source=T target=OnlyStrings.T
/// @resolution.name source=T target=OnlyStrings.T

type Result = OnlyStrings<never>;
/// @type.symbol symbol=Result source="type Result = OnlyStrings<never>" type=never
/// @definition.type symbol=Result source="type Result = OnlyStrings<never>" value=never
/// @resolution.name source=OnlyStrings target=OnlyStrings

let value: Result = "no";
/// @type.symbol symbol=value source=value type=never
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Result target=Result
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"no\"' is not assignable to type 'never'"
/// @diagnostic.label line=5 column=21 span="\"no\"" line_source="let value: Result = \"no\";"
/// @diagnostic.related line=5 column=12 span="Result" line_source="let value: Result = \"no\";" message="expected due to this annotation"
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

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Box<T> = { value: T };
type Unbox<T> = T extends Box<infer U> ? U : never;
type Value = Unbox<Box<"ready">>;

declare const value: "ready";

=== dir ===
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
/// @type.symbol symbol=Value source="type Value = Unbox<Box<\"ready\">>" type="ready"
/// @definition.type symbol=Value source="type Value = Unbox<Box<\"ready\">>" value="ready"
/// @resolution.name source=Unbox target=Unbox
/// @resolution.name source=Box target=Box

declare const value: Value;
/// @type.symbol symbol=value source=value type="ready"
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Value target=Value
"#,
    );
}

#[test]
fn test_conditional_type_infer_inherits_constructor_parameter_bound() {
    let session = TestSession::single(
        r#"
newtype Vector<T, const N: int> = intrinsic;
type LaneCount<V> = V extends Vector<infer T, infer N> ? N : never;
type Count = LaneCount<Vector<string, 4>>;

declare const count: Count;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
newtype Vector<in out T, const N: int> = intrinsic;
type LaneCount<V> = V extends Vector<infer T, infer N> ? N : never;
type Count = LaneCount<Vector<string, 4>>;

declare const count: 4;

=== dir ===
newtype Vector<T, const N: int> = intrinsic;
/// @generic.template symbol=Vector parameters=(in out T#1, const N#1: int64)
/// @type.symbol symbol=Vector source="newtype Vector<T, const N: int> = intrinsic" type=Vector
/// @definition.newtype symbol=Vector source="newtype Vector<T, const N: int> = intrinsic" template=(in out T#1, const N#1: int64) backing=intrinsic constructors=[<T#1, const N#1: int64>(intrinsic) => Vector<T#1, N#1>]
/// @type.symbol symbol=Vector.T source=T type=T#1
/// @type.symbol symbol=Vector.N source="const N: int" type=N#1

type LaneCount<V> = V extends Vector<infer T, infer N> ? N : never;
/// @generic.template symbol=LaneCount parameters=(V)
/// @type.symbol symbol=LaneCount source="type LaneCount<V> = V extends Vector<infer T, infer N> ? N : never" type=V extends Vector<infer T, infer N> ? LaneCount.N : never
/// @definition.type symbol=LaneCount source="type LaneCount<V> = V extends Vector<infer T, infer N> ? N : never" template=(V) value=V extends Vector<infer T, infer N> ? LaneCount.N : never
/// @type.symbol symbol=LaneCount.V source=V type=V
/// @resolution.name source=V target=LaneCount.V
/// @resolution.name source=Vector target=Vector
/// @generic.instance id="Vector<infer T, infer N>" template=Vector arguments=(infer T, infer N)
/// @resolution.name source=N target=LaneCount.N

type Count = LaneCount<Vector<string, 4>>;
/// @type.symbol symbol=Count source="type Count = LaneCount<Vector<string, 4>>" type=4
/// @definition.type symbol=Count source="type Count = LaneCount<Vector<string, 4>>" value=4
/// @resolution.name source=LaneCount target=LaneCount
/// @resolution.name source=Vector target=Vector
/// @generic.instance id="Vector<string, 4>" template=Vector arguments=(string, 4)

declare const count: Count;
/// @type.symbol symbol=count source=count type=4
/// @resolution.pattern source=count kind=binding target=count
/// @resolution.name source=Count target=Count
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

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Box<T> = { value: T };
type Text<T> = T extends Box<infer U extends string> ? U : never;
type Value = Text<Box<int32>>;

let value: never = "no";

=== dir ===
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
/// @type.symbol symbol=Value source="type Value = Text<Box<int32>>" type=never
/// @definition.type symbol=Value source="type Value = Text<Box<int32>>" value=never
/// @resolution.name source=Text target=Text
/// @resolution.name source=Box target=Box

let value: Value = "no";
/// @type.symbol symbol=value source=value type=never
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Value target=Value
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"no\"' is not assignable to type 'never'"
/// @diagnostic.label line=6 column=20 span="\"no\"" line_source="let value: Value = \"no\";"
/// @diagnostic.related line=6 column=12 span="Value" line_source="let value: Value = \"no\";" message="expected due to this annotation"
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

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Box<T> = { value: T };
type IsBox<T> = T extends Box<infer _> ? true : false;
type Yes = IsBox<Box<string>>;
type No = IsBox<string>;

const yes: true = true;
const no: false = false;

=== dir ===
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
/// @resolution.pattern source=yes kind=binding target=yes
/// @resolution.name source=Yes target=Yes

const no: No = false;
/// @type.symbol symbol=no source=no type=false
/// @resolution.pattern source=no kind=binding target=no
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

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Box<T> = { value: T };
type Unbox<T> = T extends Box<infer U> ? U : never;
type Value = Unbox<Box<"a"> | Box<"b">>;

const first: "a" | "b" = "a" as "a" | "b";
const second: "a" | "b" = "b" as "a" | "b";

=== dir ===
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
/// @type.symbol symbol=Value source="type Value = Unbox<Box<\"a\"> | Box<\"b\">>" type="a" | "b"
/// @definition.type symbol=Value source="type Value = Unbox<Box<\"a\"> | Box<\"b\">>" value="a" | "b"
/// @resolution.name source=Unbox target=Unbox
/// @resolution.name source=Box target=Box
/// @generic.instance id="Box<\"a\">" template=Box arguments=("a")
/// @generic.instance id="Box<\"b\">" template=Box arguments=("b")
/// @resolution.name source=Box target=Box

const first: Value = "a";
/// @type.symbol symbol=first source=first type="a" | "b"
/// @resolution.pattern source=first kind=binding target=first
/// @resolution.name source=Value target=Value

const second: Value = "b";
/// @type.symbol symbol=second source=second type="a" | "b"
/// @resolution.pattern source=second kind=binding target=second
/// @resolution.name source=Value target=Value
"#,
    );
}

/// Match a managed application against a readonly array pattern with infer.
#[test]
fn test_conditional_matches_an_array_through_a_readonly_pattern() {
    let session = TestSession::single(
        r#"
type Element<T> = T extends readonly (infer U)[] ? U : never;

declare const values: int32[][];

const first: Element<typeof values> = values[0];
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Element<T> = T extends readonly (infer U)[] ? U : never;

declare const values: int32[][];

const first: int32[] = values[0];

=== dir ===
type Element<T> = T extends readonly (infer U)[] ? U : never;
/// @generic.template symbol=Element parameters=(T)
/// @type.symbol symbol=Element source="type Element<T> = T extends readonly (infer U)[] ? U : never" type=T extends readonly infer U[] ? Element.U : never
/// @definition.type symbol=Element source="type Element<T> = T extends readonly (infer U)[] ? U : never" template=(T) value=T extends readonly infer U[] ? Element.U : never
/// @type.symbol symbol=Element.T source=T type=T
/// @resolution.name source=T target=Element.T
/// @resolution.name source=U target=Element.U

declare const values: int32[][];
/// @type.symbol symbol=values source=values type=int32[][]
/// @resolution.pattern source=values kind=binding target=values

const first: Element<typeof values> = values[0];
/// @type.symbol symbol=first source=first type=int32[]
/// @resolution.pattern source=first kind=binding target=first
/// @resolution.name source=Element target=Element
/// @resolution.name source=values target=values
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=values root=values
/// @resolution.place source=values[0] placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=values[0] root=values keys=[0]
/// @resolution.subscript source=values[0] type=int32[] kind=call target="collections.array.index#1(parameters=(isize), arguments=(provided(0) as isize), return=memory.type.WithAccess<&'static int32[], \"exclusive\">)"
/// @generic.instantiation id="collections.array.index#1<int32[], \"exclusive\">" template=collections.array.index#1 arguments=(int32[], "exclusive")
"#,
        r#"
"#,
    );
}

/// Refine the check type by the extends type inside a conditional's true branch.
#[test]
fn test_refine_the_check_type_in_the_true_branch() {
    let session = TestSession::single(
        r#"
import { Copy } from "destack:memory";

struct Holder<T: Copy> {
    value: T;
}

export type Poll<T> = T | (T extends Copy ? Holder<T> : never);
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
import { Copy } from "destack:memory";

struct Holder<out T: Copy> {
    value: T;
}

export type Poll<T> = T | (T extends Copy ? Holder<T> : never);

=== dir ===
import { Copy } from "destack:memory";

struct Holder<T: Copy> {
/// @generic.template symbol=Holder parameters=(out T#1: memory.capability.Copy)
/// @type.symbol symbol=Holder type=Holder
/// @definition.struct symbol=Holder template=(out T#1: memory.capability.Copy)
/// @definition.field symbol=Holder.value source="value: T" key=value type=T#1
/// @type.symbol symbol=Holder.T source="T: Copy" type=T#1
/// @resolution.name source=Copy target=memory.capability.Copy

    value: T;
    /// @type.symbol symbol=Holder.value source="value: T" type=T#1
    /// @resolution.name source=T target=Holder.T

}

export type Poll<T> = T | (T extends Copy ? Holder<T> : never);
/// @generic.template symbol=Poll parameters=(T#2)
/// @type.symbol symbol=Poll source="export type Poll<T> = T | (T extends Copy ? Holder<T> : never)" type=T#2 | T#2 extends memory.capability.Copy ? Holder<T#2> : never
/// @definition.type symbol=Poll source="export type Poll<T> = T | (T extends Copy ? Holder<T> : never)" template=(T#2) value=T#2 | T#2 extends memory.capability.Copy ? Holder<T#2> : never
/// @type.symbol symbol=Poll.T source=T type=T#2
/// @resolution.name source=T target=Poll.T
/// @resolution.name source=T target=Poll.T
/// @resolution.name source=Copy target=memory.capability.Copy
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=T target=Poll.T
"#, r#""#);
}

/// Reject a bound the false branch of a conditional leaves unproven.
#[test]
fn test_reject_the_refinement_in_the_false_branch() {
    let session = TestSession::single(
        r#"
import { Copy } from "destack:memory";

struct Holder<T: Copy> {
    value: T;
}

export type Poll<T> = T extends Copy ? never : Holder<T>;
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
import { Copy } from "destack:memory";

struct Holder<out T: Copy> {
    value: T;
}

export type Poll<T> = T extends Copy ? never : Holder<T>;

=== dir ===
import { Copy } from "destack:memory";

struct Holder<T: Copy> {
/// @generic.template symbol=Holder parameters=(out T#1: memory.capability.Copy)
/// @type.symbol symbol=Holder type=Holder
/// @definition.struct symbol=Holder template=(out T#1: memory.capability.Copy)
/// @definition.field symbol=Holder.value source="value: T" key=value type=T#1
/// @type.symbol symbol=Holder.T source="T: Copy" type=T#1
/// @resolution.name source=Copy target=memory.capability.Copy

    value: T;
    /// @type.symbol symbol=Holder.value source="value: T" type=T#1
    /// @resolution.name source=T target=Holder.T

}

export type Poll<T> = T extends Copy ? never : Holder<T>;
/// @generic.template symbol=Poll parameters=(T#2)
/// @type.symbol symbol=Poll source="export type Poll<T> = T extends Copy ? never : Holder<T>" type=T#2 extends memory.capability.Copy ? never : Holder<T#2>
/// @definition.type symbol=Poll source="export type Poll<T> = T extends Copy ? never : Holder<T>" template=(T#2) value=T#2 extends memory.capability.Copy ? never : Holder<T#2>
/// @type.symbol symbol=Poll.T source=T type=T#2
/// @resolution.name source=T target=Poll.T
/// @resolution.name source=Copy target=memory.capability.Copy
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=T target=Poll.T
"#, r#"
/// @diagnostic.error id=not-erasable message="type 'T' cannot be erased into 'Copy'"
/// @diagnostic.label line=8 column=55 span="T" line_source="export type Poll<T> = T extends Copy ? never : Holder<T>;"
/// @diagnostic.related line=4 column=15 span="T" line_source="struct Holder<T: Copy> {" message="required by this bound on 'T'"
/// @diagnostic.help message="prove the source erasable with a DynamicSafe bound"
"#);
}

/// Keep a concrete conditional check from becoming an assumption in its true branch.
#[test]
fn test_keep_a_concrete_check_out_of_the_true_branch() {
    let session = TestSession::single(
        r#"
import { Copy } from "destack:memory";

struct Buffer {
    text: ^string;
}

struct Holder<T: Copy> {
    value: T;
}

export type Pick = string extends usize ? Holder<Buffer> : never;
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
import { Copy } from "destack:memory";

struct Buffer {
    text: ^string;
}

struct Holder<out T: Copy> {
    value: T;
}

export type Pick = string extends usize ? Holder<Buffer> : never;

=== dir ===
import { Copy } from "destack:memory";

struct Buffer {
/// @type.symbol symbol=Buffer type=Buffer
/// @definition.struct symbol=Buffer
/// @definition.field symbol=Buffer.text source="text: ^string" key=text type=Owned<string>

    text: ^string;
    /// @type.symbol symbol=Buffer.text source="text: ^string" type=Owned<string>

}

struct Holder<T: Copy> {
/// @generic.template symbol=Holder parameters=(out T: memory.capability.Copy)
/// @type.symbol symbol=Holder type=Holder
/// @definition.struct symbol=Holder template=(out T: memory.capability.Copy)
/// @definition.field symbol=Holder.value source="value: T" key=value type=T
/// @type.symbol symbol=Holder.T source="T: Copy" type=T
/// @resolution.name source=Copy target=memory.capability.Copy

    value: T;
    /// @type.symbol symbol=Holder.value source="value: T" type=T
    /// @resolution.name source=T target=Holder.T

}

export type Pick = string extends usize ? Holder<Buffer> : never;
/// @type.symbol symbol=Pick source="export type Pick = string extends usize ? Holder<Buffer> : never" type=never
/// @definition.type symbol=Pick source="export type Pick = string extends usize ? Holder<Buffer> : never" value=never
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=Buffer target=Buffer
"#, r#"
/// @diagnostic.error id=constraint-not-satisfied message="type 'Buffer' does not satisfy 'Copy'"
/// @diagnostic.label line=12 column=50 span="Buffer" line_source="export type Pick = string extends usize ? Holder<Buffer> : never;"
/// @diagnostic.related line=8 column=15 span="T" line_source="struct Holder<T: Copy> {" message="required by this bound on 'T'"
"#);
}
