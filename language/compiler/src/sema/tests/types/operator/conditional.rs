use crate::tests::{DirRows, TestSession};

/// A conditional selects a branch per check type it is applied to.
#[test]
fn test_select_a_conditional_branch_per_check_type() {
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
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Select<T> = T extends string ? "yes" : "no";
type Text = Select<string>;
type Number = Select<int32>;

declare const text: Text;
declare const number: Number;

=== dir ===
type Select<T> = T extends string ? "yes" : "no";
/// @generic.template symbol=Select parameters=(T)
/// @type.symbol symbol=Select source="type Select<T> = T extends string ? \"yes\" : \"no\"" type=T extends string ? "yes" : "no"
/// @definition.type symbol=Select source="type Select<T> = T extends string ? \"yes\" : \"no\"" template=(T) value=T extends string ? "yes" : "no"
/// @type.symbol symbol=Select.T source=T type=T
/// @resolution.name source=T target=Select.T

type Text = Select<string>;
/// @type.symbol symbol=Text source="type Text = Select<string>" type="yes"
/// @definition.type symbol=Text source="type Text = Select<string>" value=Select<string>
/// @resolution.name source=Select target=Select

type Number = Select<int32>;
/// @type.symbol symbol=Number source="type Number = Select<int32>" type="no"
/// @definition.type symbol=Number source="type Number = Select<int32>" value=Select<int32>
/// @resolution.name source=Select target=Select

declare const text: Text;
/// @type.symbol symbol=text source=text type=Text
/// @resolution.pattern source=text kind=binding target=text
/// @resolution.name source=Text target=Text

declare const number: Number;
/// @type.symbol symbol=number source=number type=Number
/// @resolution.pattern source=number kind=binding target=number
/// @resolution.name source=Number target=Number
"#,
    );
}

/// A conditional over a naked parameter distributes across a union check type.
#[test]
fn test_distribute_a_conditional_over_a_naked_parameter() {
    let session = TestSession::single(
        r#"
type OnlyStrings<T> = T extends string ? T : never;
type Result = OnlyStrings<string | int32>;

declare const value: Result;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type OnlyStrings<T> = T extends string ? T : never;
type Result = OnlyStrings<string | int32>;

declare const value: Result;

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
/// @definition.type symbol=Result source="type Result = OnlyStrings<string | int32>" value=OnlyStrings<string | int32>
/// @resolution.name source=OnlyStrings target=OnlyStrings

declare const value: Result;
/// @type.symbol symbol=value source=value type=Result
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Result target=Result
"#,
    );
}

/// A tuple-wrapped check type keeps a conditional undistributed.
#[test]
fn test_keep_a_tuple_wrapped_check_type_undistributed() {
    let session = TestSession::single(
        r#"
type Wrapped<T> = (T,) extends (string,) ? "yes" : "no";
type Result = Wrapped<string | int32>;

declare const value: Result;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Wrapped<T> = (T,) extends (string,) ? "yes" : "no";
type Result = Wrapped<string | int32>;

declare const value: Result;

=== dir ===
type Wrapped<T> = (T,) extends (string,) ? "yes" : "no";
/// @generic.template symbol=Wrapped parameters=(T)
/// @type.symbol symbol=Wrapped source="type Wrapped<T> = (T,) extends (string,) ? \"yes\" : \"no\"" type="no"
/// @definition.type symbol=Wrapped source="type Wrapped<T> = (T,) extends (string,) ? \"yes\" : \"no\"" template=(T) value=(T,) extends (string,) ? "yes" : "no"
/// @type.symbol symbol=Wrapped.T source=T type=T
/// @resolution.name source=T target=Wrapped.T

type Result = Wrapped<string | int32>;
/// @type.symbol symbol=Result source="type Result = Wrapped<string | int32>" type="no"
/// @definition.type symbol=Result source="type Result = Wrapped<string | int32>" value=Wrapped<string | int32>
/// @resolution.name source=Wrapped target=Wrapped

declare const value: Result;
/// @type.symbol symbol=value source=value type=Result
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Result target=Result
"#,
    );
}

/// A distributed conditional over never reduces to never.
#[test]
fn test_distribute_never_to_never() {
    let session = TestSession::single(
        r#"
type OnlyStrings<T> = T extends string ? T : never;
type Result = OnlyStrings<never>;

let value: Result = "no";
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type OnlyStrings<T> = T extends string ? T : never;
type Result = OnlyStrings<never>;

let value: Result = "no";

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
/// @definition.type symbol=Result source="type Result = OnlyStrings<never>" value=OnlyStrings<never>
/// @resolution.name source=OnlyStrings target=OnlyStrings

let value: Result = "no";
/// @type.symbol symbol=value source=value type=Result
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Result target=Result
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"no\"' is not assignable to type 'Result'"
/// @diagnostic.label line=5 column=21 span="\"no\"" line_source="let value: Result = \"no\";"
/// @diagnostic.related line=5 column=12 span="Result" line_source="let value: Result = \"no\";" message="expected due to this annotation"
/// @diagnostic.note message="'Result' reduces to 'never'"
"#,
    );
}

/// An infer binder captures the argument of a type application.
#[test]
fn test_capture_an_application_argument() {
    let session = TestSession::single(
        r#"
type Box<T> = { value: T };
type Unbox<T> = T extends Box<infer U> ? U : never;
type Value = Unbox<Box<"ready">>;

declare const value: Value;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Box<T> = { value: T };
type Unbox<T> = T extends Box<infer U> ? U : never;
type Value = Unbox<Box<"ready">>;

declare const value: Value;

=== dir ===
type Box<T> = { value: T };
/// @generic.template symbol=Box parameters=(T#1)
/// @type.symbol symbol=Box source="type Box<T> = { value: T }" type={ value: T#1 }
/// @definition.type symbol=Box source="type Box<T> = { value: T }" template=(T#1) value={ value: T#1 }
/// @type.symbol symbol=Box.T source=T type=T#1
/// @type.symbol symbol=Box.value source="value: T" type=T#1
/// @resolution.name source=T target=Box.T

type Unbox<T> = T extends Box<infer U> ? U : never;
/// @generic.template symbol=Unbox parameters=(T#2)
/// @type.symbol symbol=Unbox source="type Unbox<T> = T extends Box<infer U> ? U : never" type=T#2 extends { value: infer U } ? Unbox.U : never
/// @definition.type symbol=Unbox source="type Unbox<T> = T extends Box<infer U> ? U : never" template=(T#2) value=T#2 extends Box<infer U> ? Unbox.U : never
/// @type.symbol symbol=Unbox.T source=T type=T#2
/// @resolution.name source=T target=Unbox.T
/// @resolution.name source=Box target=Box
/// @resolution.name source=U target=Unbox.U

type Value = Unbox<Box<"ready">>;
/// @type.symbol symbol=Value source="type Value = Unbox<Box<\"ready\">>" type="ready"
/// @generic.instance id="Box<\"ready\">" template=Box arguments=("ready")
/// @definition.type symbol=Value source="type Value = Unbox<Box<\"ready\">>" value=Unbox<Box<"ready">>
/// @resolution.name source=Unbox target=Unbox
/// @resolution.name source=Box target=Box

declare const value: Value;
/// @type.symbol symbol=value source=value type=Value
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Value target=Value
"#,
    );
}

/// An infer binder captures the argument of a const type parameter.
#[test]
fn test_capture_a_const_parameter_argument() {
    let session = TestSession::single(
        r#"
newtype Vector<T, const N: int> = intrinsic;
type LaneCount<V> = V extends Vector<infer T, infer N> ? N : never;
type Count = LaneCount<Vector<string, 4>>;

declare const count: Count;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
newtype Vector<in out T, const N: int> = intrinsic;
type LaneCount<V> = V extends Vector<infer T, infer N> ? N : never;
type Count = LaneCount<Vector<string, 4>>;

declare const count: Count;

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
/// @resolution.name source=N target=LaneCount.N

type Count = LaneCount<Vector<string, 4>>;
/// @type.symbol symbol=Count source="type Count = LaneCount<Vector<string, 4>>" type=4
/// @generic.instance id="Vector<string, 4>" template=Vector arguments=(string, 4)
/// @definition.type symbol=Count source="type Count = LaneCount<Vector<string, 4>>" value=LaneCount<Vector<string, 4>>
/// @resolution.name source=LaneCount target=LaneCount
/// @resolution.name source=Vector target=Vector

declare const count: Count;
/// @type.symbol symbol=count source=count type=Count
/// @resolution.pattern source=count kind=binding target=count
/// @resolution.name source=Count target=Count
"#,
    );
}

/// A capture failing its infer constraint takes the false branch.
#[test]
fn test_reject_a_capture_that_fails_its_infer_constraint() {
    let session = TestSession::single(
        r#"
type Box<T> = { value: T };
type Text<T> = T extends Box<infer U extends string> ? U : never;
type Value = Text<Box<int32>>;

let value: Value = "no";
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Box<T> = { value: T };
type Text<T> = T extends Box<infer U extends string> ? U : never;
type Value = Text<Box<int32>>;

let value: Value = "no";

=== dir ===
type Box<T> = { value: T };
/// @generic.template symbol=Box parameters=(T#1)
/// @type.symbol symbol=Box source="type Box<T> = { value: T }" type={ value: T#1 }
/// @definition.type symbol=Box source="type Box<T> = { value: T }" template=(T#1) value={ value: T#1 }
/// @type.symbol symbol=Box.T source=T type=T#1
/// @type.symbol symbol=Box.value source="value: T" type=T#1
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
/// @definition.type symbol=Value source="type Value = Text<Box<int32>>" value=Text<Box<int32>>
/// @resolution.name source=Text target=Text
/// @resolution.name source=Box target=Box

let value: Value = "no";
/// @type.symbol symbol=value source=value type=Value
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Value target=Value
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"no\"' is not assignable to type 'Value'"
/// @diagnostic.label line=6 column=20 span="\"no\"" line_source="let value: Value = \"no\";"
/// @diagnostic.related line=6 column=12 span="Value" line_source="let value: Value = \"no\";" message="expected due to this annotation"
/// @diagnostic.note message="'Value' reduces to 'never'"
"#,
    );
}

/// An anonymous infer binder matches without naming the captured type.
#[test]
fn test_match_an_anonymous_infer_binder() {
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
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Box<T> = { value: T };
type IsBox<T> = T extends Box<infer _> ? true : false;
type Yes = IsBox<Box<string>>;
type No = IsBox<string>;

const yes: Yes = true;
const no: No = false;

=== dir ===
type Box<T> = { value: T };
/// @generic.template symbol=Box parameters=(T#1)
/// @type.symbol symbol=Box source="type Box<T> = { value: T }" type={ value: T#1 }
/// @definition.type symbol=Box source="type Box<T> = { value: T }" template=(T#1) value={ value: T#1 }
/// @type.symbol symbol=Box.T source=T type=T#1
/// @type.symbol symbol=Box.value source="value: T" type=T#1
/// @resolution.name source=T target=Box.T

type IsBox<T> = T extends Box<infer _> ? true : false;
/// @generic.template symbol=IsBox parameters=(T#2)
/// @type.symbol symbol=IsBox source="type IsBox<T> = T extends Box<infer _> ? true : false" type=T#2 extends { value: infer _ } ? true : false
/// @definition.type symbol=IsBox source="type IsBox<T> = T extends Box<infer _> ? true : false" template=(T#2) value=T#2 extends Box<infer _> ? true : false
/// @type.symbol symbol=IsBox.T source=T type=T#2
/// @resolution.name source=T target=IsBox.T
/// @resolution.name source=Box target=Box

type Yes = IsBox<Box<string>>;
/// @type.symbol symbol=Yes source="type Yes = IsBox<Box<string>>" type=true
/// @generic.instance id=Box<string> template=Box arguments=(string)
/// @definition.type symbol=Yes source="type Yes = IsBox<Box<string>>" value=IsBox<Box<string>>
/// @resolution.name source=IsBox target=IsBox
/// @resolution.name source=Box target=Box

type No = IsBox<string>;
/// @type.symbol symbol=No source="type No = IsBox<string>" type=false
/// @definition.type symbol=No source="type No = IsBox<string>" value=IsBox<string>
/// @resolution.name source=IsBox target=IsBox

const yes: Yes = true;
/// @type.symbol symbol=yes source=yes type=Yes
/// @resolution.pattern source=yes kind=binding target=yes
/// @resolution.name source=Yes target=Yes

const no: No = false;
/// @type.symbol symbol=no source=no type=No
/// @resolution.pattern source=no kind=binding target=no
/// @resolution.name source=No target=No
"#,
    );
}

/// A distributed conditional captures one infer binding per union element.
#[test]
fn test_capture_per_distributed_union_element() {
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
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Box<T> = { value: T };
type Unbox<T> = T extends Box<infer U> ? U : never;
type Value = Unbox<Box<"a"> | Box<"b">>;

const first: Value = "a";
const second: Value = "b";

=== dir ===
type Box<T> = { value: T };
/// @generic.template symbol=Box parameters=(T#1)
/// @type.symbol symbol=Box source="type Box<T> = { value: T }" type={ value: T#1 }
/// @definition.type symbol=Box source="type Box<T> = { value: T }" template=(T#1) value={ value: T#1 }
/// @type.symbol symbol=Box.T source=T type=T#1
/// @type.symbol symbol=Box.value source="value: T" type=T#1
/// @resolution.name source=T target=Box.T

type Unbox<T> = T extends Box<infer U> ? U : never;
/// @generic.template symbol=Unbox parameters=(T#2)
/// @type.symbol symbol=Unbox source="type Unbox<T> = T extends Box<infer U> ? U : never" type=T#2 extends { value: infer U } ? Unbox.U : never
/// @definition.type symbol=Unbox source="type Unbox<T> = T extends Box<infer U> ? U : never" template=(T#2) value=T#2 extends Box<infer U> ? Unbox.U : never
/// @type.symbol symbol=Unbox.T source=T type=T#2
/// @resolution.name source=T target=Unbox.T
/// @resolution.name source=Box target=Box
/// @resolution.name source=U target=Unbox.U

type Value = Unbox<Box<"a"> | Box<"b">>;
/// @type.symbol symbol=Value source="type Value = Unbox<Box<\"a\"> | Box<\"b\">>" type="a" | "b"
/// @generic.instance id="Box<\"a\">" template=Box arguments=("a")
/// @generic.instance id="Box<\"b\">" template=Box arguments=("b")
/// @definition.type symbol=Value source="type Value = Unbox<Box<\"a\"> | Box<\"b\">>" value=Unbox<Box<"a"> | Box<"b">>
/// @resolution.name source=Unbox target=Unbox
/// @resolution.name source=Box target=Box
/// @resolution.name source=Box target=Box

const first: Value = "a";
/// @type.symbol symbol=first source=first type=Value
/// @resolution.pattern source=first kind=binding target=first
/// @resolution.name source=Value target=Value

const second: Value = "b";
/// @type.symbol symbol=second source=second type=Value
/// @resolution.pattern source=second kind=binding target=second
/// @resolution.name source=Value target=Value
"#,
    );
}

/// Match a managed application against a readonly array pattern with infer.
#[test]
fn test_match_a_managed_array_through_a_readonly_pattern() {
    let session = TestSession::single(
        r#"
type Element<T> = T extends readonly (infer U)[] ? U : never;

declare const values: int32[][];

const first: Element<typeof values> = values[0];
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Element<T> = T extends readonly (infer U)[] ? U : never;

declare const values: int32[][];

const first: Element<int32[][]> = values[0];

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
/// @type.symbol symbol=first source=first type=Element<int32[][]>
/// @resolution.pattern source=first kind=binding target=first
/// @resolution.name source=Element target=Element
/// @resolution.name source=values target=values
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="local" lifetime="static" access="immutable"
/// @resolution.access source=values root=values
/// @resolution.subscript source=values[0] type=int32[] kind=call target="index#2(parameters=(isize), arguments=(provided(0) as isize), return=int32[], regions=(\"managed\" & \"local\"))"
/// @generic.instantiation id="index#2<int32[], \"managed\" & \"local\">" template=index#2 arguments=(int32[], "managed" & "local")
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
import { Copy } from "tspp:memory";

struct Holder<T: Copy> {
    value: T;
}

export type Poll<T> = T | (T extends Copy ? Holder<T> : never);
"#,
    );

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
import { Copy } from "tspp:memory";

struct Holder<out T: Copy> {
    value: T;
}

export type Poll<T> = T | (T extends Copy ? Holder<T> : never);

=== dir ===
import { Copy } from "tspp:memory";

struct Holder<T: Copy> {
/// @generic.template symbol=Holder parameters=(out T#1: Copy)
/// @type.symbol symbol=Holder type=Holder
/// @definition.struct symbol=Holder template=(out T#1: Copy)
/// @definition.field symbol=Holder.value source="value: T" key=value type=T#1
/// @type.symbol symbol=Holder.T source="T: Copy" type=T#1
/// @resolution.name source=Copy target=Copy

    value: T;
    /// @type.symbol symbol=Holder.value source="value: T" type=T#1
    /// @resolution.name source=T target=Holder.T

}

export type Poll<T> = T | (T extends Copy ? Holder<T> : never);
/// @generic.template symbol=Poll parameters=(T#2)
/// @type.symbol symbol=Poll source="export type Poll<T> = T | (T extends Copy ? Holder<T> : never)" type=T#2 | T#2 extends Copy ? Holder<T#2> : never
/// @definition.type symbol=Poll source="export type Poll<T> = T | (T extends Copy ? Holder<T> : never)" template=(T#2) value=T#2 | T#2 extends Copy ? Holder<T#2> : never
/// @type.symbol symbol=Poll.T source=T type=T#2
/// @resolution.name source=T target=Poll.T
/// @resolution.name source=T target=Poll.T
/// @resolution.name source=Copy target=Copy
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=T target=Poll.T
"#, r#"
"#);
}

/// Reject a bound the false branch of a conditional leaves unproven.
#[test]
fn test_reject_the_refinement_in_the_false_branch() {
    let session = TestSession::single(
        r#"
import { Copy } from "tspp:memory";

struct Holder<T: Copy> {
    value: T;
}

export type Poll<T> = T extends Copy ? never : Holder<T>;
"#,
    );

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
import { Copy } from "tspp:memory";

struct Holder<out T: Copy> {
    value: T;
}

export type Poll<T> = T extends Copy ? never : Holder<T>;

=== dir ===
import { Copy } from "tspp:memory";

struct Holder<T: Copy> {
/// @generic.template symbol=Holder parameters=(out T#1: Copy)
/// @type.symbol symbol=Holder type=Holder
/// @definition.struct symbol=Holder template=(out T#1: Copy)
/// @definition.field symbol=Holder.value source="value: T" key=value type=T#1
/// @type.symbol symbol=Holder.T source="T: Copy" type=T#1
/// @resolution.name source=Copy target=Copy

    value: T;
    /// @type.symbol symbol=Holder.value source="value: T" type=T#1
    /// @resolution.name source=T target=Holder.T

}

export type Poll<T> = T extends Copy ? never : Holder<T>;
/// @generic.template symbol=Poll parameters=(T#2)
/// @type.symbol symbol=Poll source="export type Poll<T> = T extends Copy ? never : Holder<T>" type=T#2 extends Copy ? never : Holder<T#2>
/// @definition.type symbol=Poll source="export type Poll<T> = T extends Copy ? never : Holder<T>" template=(T#2) value=T#2 extends Copy ? never : Holder<T#2>
/// @type.symbol symbol=Poll.T source=T type=T#2
/// @resolution.name source=T target=Poll.T
/// @resolution.name source=Copy target=Copy
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=T target=Poll.T
"#, r#"
/// @diagnostic.error id=constraint-not-satisfied message="type 'T' does not satisfy 'Copy'"
/// @diagnostic.label line=8 column=55 span="T" line_source="export type Poll<T> = T extends Copy ? never : Holder<T>;"
/// @diagnostic.related line=4 column=15 span="T" line_source="struct Holder<T: Copy> {" message="required by this bound on 'T'"
"#);
}

/// Keep a concrete conditional check from becoming an assumption in its true branch.
#[test]
fn test_keep_a_concrete_check_out_of_the_true_branch() {
    let session = TestSession::single(
        r#"
import { Copy } from "tspp:memory";

struct Buffer {
    text: ^string;
}

struct Holder<T: Copy> {
    value: T;
}

export type Pick = string extends usize ? Holder<Buffer> : never;
"#,
    );

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
import { Copy } from "tspp:memory";

struct Buffer {
    text: ^string;
}

struct Holder<out T: Copy> {
    value: T;
}

export type Pick = string extends usize ? Holder<Buffer> : never;

=== dir ===
import { Copy } from "tspp:memory";

struct Buffer {
/// @type.symbol symbol=Buffer type=Buffer
/// @definition.struct symbol=Buffer
/// @definition.field symbol=Buffer.text source="text: ^string" key=text type=^string

    text: ^string;
    /// @type.symbol symbol=Buffer.text source="text: ^string" type=^string

}

struct Holder<T: Copy> {
/// @generic.template symbol=Holder parameters=(out T: Copy)
/// @type.symbol symbol=Holder type=Holder
/// @definition.struct symbol=Holder template=(out T: Copy)
/// @definition.field symbol=Holder.value source="value: T" key=value type=T
/// @type.symbol symbol=Holder.T source="T: Copy" type=T
/// @resolution.name source=Copy target=Copy

    value: T;
    /// @type.symbol symbol=Holder.value source="value: T" type=T
    /// @resolution.name source=T target=Holder.T

}

export type Pick = string extends usize ? Holder<Buffer> : never;
/// @type.symbol symbol=Pick source="export type Pick = string extends usize ? Holder<Buffer> : never" type=never
/// @definition.type symbol=Pick source="export type Pick = string extends usize ? Holder<Buffer> : never" value=string extends usize ? Holder<Buffer> : never
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=Buffer target=Buffer
"#, r#"
/// @diagnostic.error id=constraint-not-satisfied message="type 'Buffer' does not satisfy 'Copy'"
/// @diagnostic.label line=12 column=50 span="Buffer" line_source="export type Pick = string extends usize ? Holder<Buffer> : never;"
/// @diagnostic.related line=8 column=15 span="T" line_source="struct Holder<T: Copy> {" message="required by this bound on 'T'"
"#);
}

/// Take the true branch for a wrapped `never` check, which distribution would skip.
#[test]
fn test_take_the_true_branch_for_a_wrapped_never_check() {
    let session = TestSession::single(
        r#"
type IsString<T> = (T,) extends (string,) ? true : false;
type Result = IsString<never>;

declare const value: Result;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type IsString<T> = (T,) extends (string,) ? true : false;
type Result = IsString<never>;

declare const value: Result;

=== dir ===
type IsString<T> = (T,) extends (string,) ? true : false;
/// @generic.template symbol=IsString parameters=(T)
/// @type.symbol symbol=IsString source="type IsString<T> = (T,) extends (string,) ? true : false" type=false
/// @definition.type symbol=IsString source="type IsString<T> = (T,) extends (string,) ? true : false" template=(T) value=(T,) extends (string,) ? true : false
/// @type.symbol symbol=IsString.T source=T type=T
/// @resolution.name source=T target=IsString.T

type Result = IsString<never>;
/// @type.symbol symbol=Result source="type Result = IsString<never>" type=true
/// @definition.type symbol=Result source="type Result = IsString<never>" value=IsString<never>
/// @resolution.name source=IsString target=IsString

declare const value: Result;
/// @type.symbol symbol=value source=value type=Result
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Result target=Result
"#,
        r#"
"#,
    );
}

/// Split a tuple into its head element and its rest elements.
#[test]
fn test_split_a_tuple_into_its_head_and_rest_elements() {
    let session = TestSession::single(
        r#"
type Head<T> = T extends (infer H, ...infer R) ? H : "no";
type Rest<T> = T extends (infer H, ...infer R) ? R : "no";

declare const head: Head<(1, 2, 3)>;
declare const rest: Rest<(1, 2, 3)>;
declare const empty: Head<()>;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Head<T> = T extends (infer H, ...infer R) ? H : "no";
type Rest<T> = T extends (infer H, ...infer R) ? R : "no";

declare const head: Head<(1, 2, 3)>;
declare const rest: Rest<(1, 2, 3)>;
declare const empty: Head<()>;

=== dir ===
type Head<T> = T extends (infer H, ...infer R) ? H : "no";
/// @generic.template symbol=Head parameters=(T#1)
/// @type.symbol symbol=Head source="type Head<T> = T extends (infer H, ...infer R) ? H : \"no\"" type=T#1 extends (infer H, ...infer R extends (readonly ...ReadonlyArray<unknown>,)) ? Head.H : "no"
/// @definition.type symbol=Head source="type Head<T> = T extends (infer H, ...infer R) ? H : \"no\"" template=(T#1) value=T#1 extends (infer H, ...infer R extends (readonly ...ReadonlyArray<unknown>,)) ? Head.H : "no"
/// @type.symbol symbol=Head.T source=T type=T#1
/// @resolution.name source=T target=Head.T
/// @resolution.name source=H target=Head.H

type Rest<T> = T extends (infer H, ...infer R) ? R : "no";
/// @generic.template symbol=Rest parameters=(T#2)
/// @type.symbol symbol=Rest source="type Rest<T> = T extends (infer H, ...infer R) ? R : \"no\"" type=T#2 extends (infer H, ...infer R extends (readonly ...ReadonlyArray<unknown>,)) ? Rest.R : "no"
/// @definition.type symbol=Rest source="type Rest<T> = T extends (infer H, ...infer R) ? R : \"no\"" template=(T#2) value=T#2 extends (infer H, ...infer R extends (readonly ...ReadonlyArray<unknown>,)) ? Rest.R : "no"
/// @type.symbol symbol=Rest.T source=T type=T#2
/// @resolution.name source=T target=Rest.T
/// @resolution.name source=R target=Rest.R

declare const head: Head<(1, 2, 3)>;
/// @type.symbol symbol=head source=head type=Head<(1, 2, 3)>
/// @resolution.pattern source=head kind=binding target=head
/// @resolution.name source=Head target=Head

declare const rest: Rest<(1, 2, 3)>;
/// @type.symbol symbol=rest source=rest type=Rest<(1, 2, 3)>
/// @resolution.pattern source=rest kind=binding target=rest
/// @resolution.name source=Rest target=Rest

declare const empty: Head<()>;
/// @type.symbol symbol=empty source=empty type=Head<()>
/// @resolution.pattern source=empty kind=binding target=empty
/// @resolution.name source=Head target=Head
"#,
        r#"
"#,
    );
}

/// Capture the last element of a tuple behind a leading rest pattern.
#[test]
fn test_capture_the_last_tuple_element_behind_a_leading_rest() {
    let session = TestSession::single(
        r#"
type Last<T> = T extends (...infer I, infer L) ? L : "no";

declare const last: Last<(1, 2, 3)>;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Last<T> = T extends (...infer I, infer L) ? L : "no";

declare const last: Last<(1, 2, 3)>;

=== dir ===
type Last<T> = T extends (...infer I, infer L) ? L : "no";
/// @generic.template symbol=Last parameters=(T)
/// @type.symbol symbol=Last source="type Last<T> = T extends (...infer I, infer L) ? L : \"no\"" type=T extends (...infer I extends (readonly ...ReadonlyArray<unknown>,), infer L) ? Last.L : "no"
/// @definition.type symbol=Last source="type Last<T> = T extends (...infer I, infer L) ? L : \"no\"" template=(T) value=T extends (...infer I extends (readonly ...ReadonlyArray<unknown>,), infer L) ? Last.L : "no"
/// @type.symbol symbol=Last.T source=T type=T
/// @resolution.name source=T target=Last.T
/// @resolution.name source=L target=Last.L

declare const last: Last<(1, 2, 3)>;
/// @type.symbol symbol=last source=last type=Last<(1, 2, 3)>
/// @resolution.pattern source=last kind=binding target=last
/// @resolution.name source=Last target=Last
"#,
        r#"
"#,
    );
}

/// Split a readonly tuple through the same head and rest pattern.
#[test]
fn test_split_a_readonly_tuple_into_head_and_rest() {
    let session = TestSession::single(
        r#"
type Head<T> = T extends readonly (infer H, ...infer R) ? H : "no";

declare const head: Head<readonly (1, 2)>;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Head<T> = T extends readonly (infer H, ...infer R) ? H : "no";

declare const head: Head<readonly (1, 2)>;

=== dir ===
type Head<T> = T extends readonly (infer H, ...infer R) ? H : "no";
/// @generic.template symbol=Head parameters=(T)
/// @type.symbol symbol=Head source="type Head<T> = T extends readonly (infer H, ...infer R) ? H : \"no\"" type=T extends readonly (infer H, ...infer R extends (readonly ...ReadonlyArray<unknown>,)) ? Head.H : "no"
/// @definition.type symbol=Head source="type Head<T> = T extends readonly (infer H, ...infer R) ? H : \"no\"" template=(T) value=T extends readonly (infer H, ...infer R extends (readonly ...ReadonlyArray<unknown>,)) ? Head.H : "no"
/// @type.symbol symbol=Head.T source=T type=T
/// @resolution.name source=T target=Head.T
/// @resolution.name source=H target=Head.H

declare const head: Head<readonly (1, 2)>;
/// @type.symbol symbol=head source=head type=Head<readonly (1, 2)>
/// @resolution.pattern source=head kind=binding target=head
/// @resolution.name source=Head target=Head
"#,
        r#"
"#,
    );
}

/// Capture the parameter tuple and the return type of a signature.
#[test]
fn test_capture_the_parameters_and_return_type_of_a_signature() {
    let session = TestSession::single(
        r#"
type Arguments<F> = F extends (...arguments: infer P) => unknown ? P : never;
type Result<F> = F extends (...arguments: unknown[]) => infer R ? R : never;
type Signature = (name: string, count: int32) => boolean;

declare const arguments: Arguments<Signature>;
declare const result: Result<Signature>;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Arguments<F> = F extends (...arguments: infer P) => unknown ? P : never;
type Result<F> = F extends (...arguments: unknown[]) => infer R ? R : never;
type Signature = (name: string, count: int32) => boolean;

declare const arguments: Arguments<Signature>;
declare const result: Result<Signature>;

=== dir ===
type Arguments<F> = F extends (...arguments: infer P) => unknown ? P : never;
/// @generic.template symbol=Arguments parameters=(F#1)
/// @type.symbol symbol=Arguments source="type Arguments<F> = F extends (...arguments: infer P) => unknown ? P : never" type=F#1 extends (...infer P extends (readonly ...ReadonlyArray<unknown>,)) => unknown ? Arguments.P : never
/// @definition.type symbol=Arguments source="type Arguments<F> = F extends (...arguments: infer P) => unknown ? P : never" template=(F#1) value=F#1 extends (...infer P extends (readonly ...ReadonlyArray<unknown>,)) => unknown ? Arguments.P : never
/// @type.symbol symbol=Arguments.F source=F type=F#1
/// @resolution.name source=F target=Arguments.F
/// @type.symbol symbol=Arguments.arguments source="...arguments: infer P" type=infer P extends (readonly ...ReadonlyArray<unknown>,)
/// @resolution.name source=P target=Arguments.P

type Result<F> = F extends (...arguments: unknown[]) => infer R ? R : never;
/// @generic.template symbol=Result parameters=(F#2)
/// @type.symbol symbol=Result source="type Result<F> = F extends (...arguments: unknown[]) => infer R ? R : never" type=F#2 extends (...unknown[]) => infer R ? Result.R : never
/// @definition.type symbol=Result source="type Result<F> = F extends (...arguments: unknown[]) => infer R ? R : never" template=(F#2) value=F#2 extends (...unknown[]) => infer R ? Result.R : never
/// @type.symbol symbol=Result.F source=F type=F#2
/// @resolution.name source=F target=Result.F
/// @type.symbol symbol=Result.arguments source="...arguments: unknown[]" type=unknown[]
/// @resolution.name source=R target=Result.R

type Signature = (name: string, count: int32) => boolean;
/// @type.symbol symbol=Signature source="type Signature = (name: string, count: int32) => boolean" type=(string, int32) => boolean
/// @definition.type symbol=Signature source="type Signature = (name: string, count: int32) => boolean" value=(string, int32) => boolean
/// @type.symbol symbol=Signature.name source="name: string" type=string
/// @type.symbol symbol=Signature.count source="count: int32" type=int32

declare const arguments: Arguments<Signature>;
/// @type.symbol symbol=arguments source=arguments type=Arguments<Signature>
/// @resolution.pattern source=arguments kind=binding target=arguments
/// @resolution.name source=Arguments target=Arguments
/// @resolution.name source=Signature target=Signature

declare const result: Result<Signature>;
/// @type.symbol symbol=result source=result type=Result<Signature>
/// @resolution.pattern source=result kind=binding target=result
/// @resolution.name source=Result target=Result
/// @resolution.name source=Signature target=Signature
"#,
        r#"
"#,
    );
}

/// Capture the receiver parameter of a signature.
#[test]
fn test_capture_the_receiver_parameter_of_a_signature() {
    let session = TestSession::single(
        r#"
type Receiver<F> = F extends (this: infer R, ...arguments: unknown[]) => unknown ? R : never;

declare const receiver: Receiver<(this: { id: string }, count: int32) => void>;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Receiver<F> = F extends (this: infer R, ...arguments: unknown[]) => unknown ? R : never;

declare const receiver: Receiver<(this: { id: string }, count: int32) => void>;

=== dir ===
type Receiver<F> = F extends (this: infer R, ...arguments: unknown[]) => unknown ? R : never;
/// @generic.template symbol=Receiver parameters=(F)
/// @type.symbol symbol=Receiver type=F extends (this: infer R, ...unknown[]) => unknown ? Receiver.R : never
/// @definition.type symbol=Receiver template=(F) value=F extends (this: infer R, ...unknown[]) => unknown ? Receiver.R : never
/// @type.symbol symbol=Receiver.F source=F type=F
/// @resolution.name source=F target=Receiver.F
/// @type.symbol symbol=Receiver.arguments source="...arguments: unknown[]" type=unknown[]
/// @resolution.name source=R target=Receiver.R

declare const receiver: Receiver<(this: { id: string }, count: int32) => void>;
/// @type.symbol symbol=receiver source=receiver type=Receiver<(this: { id: string }, int32) => void>
/// @resolution.pattern source=receiver kind=binding target=receiver
/// @resolution.name source=Receiver target=Receiver
/// @type.symbol symbol=id source="id: string" type=string
/// @type.symbol symbol=count source="count: int32" type=int32
"#,
        r#"
"#,
    );
}

/// Capture the value type of an index signature.
#[test]
fn test_capture_the_value_type_of_an_index_signature() {
    let session = TestSession::single(
        r#"
type Values<T> = T extends { [key: string]: infer V } ? V : never;

declare const value: Values<{ [key: string]: int32 }>;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Values<T> = T extends { [key: string]: infer V } ? V : never;

declare const value: Values<{ [key: string]: int32 }>;

=== dir ===
type Values<T> = T extends { [key: string]: infer V } ? V : never;
/// @generic.template symbol=Values parameters=(T)
/// @type.symbol symbol=Values source="type Values<T> = T extends { [key: string]: infer V } ? V : never" type=T extends { [key: string]: infer V } ? Values.V : never
/// @definition.type symbol=Values source="type Values<T> = T extends { [key: string]: infer V } ? V : never" template=(T) value=T extends { [key: string]: infer V } ? Values.V : never
/// @type.symbol symbol=Values.T source=T type=T
/// @resolution.name source=T target=Values.T
/// @resolution.name source=V target=Values.V

declare const value: Values<{ [key: string]: int32 }>;
/// @type.symbol symbol=value source=value type=Values<{ [key: string]: int32 }>
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Values target=Values
"#,
        r#"
"#,
    );
}

/// Accept a tuple element that satisfies its infer constraint.
#[test]
fn test_accept_a_tuple_element_that_satisfies_its_infer_constraint() {
    let session = TestSession::single(
        r#"
type Text<T> = T extends (infer S extends string,) ? S : never;

declare const ok: Text<("a",)>;
declare const bad: Text<(1,)>;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Text<T> = T extends (infer S extends string,) ? S : never;

declare const ok: Text<("a",)>;
declare const bad: Text<(1,)>;

=== dir ===
type Text<T> = T extends (infer S extends string,) ? S : never;
/// @generic.template symbol=Text parameters=(T)
/// @type.symbol symbol=Text source="type Text<T> = T extends (infer S extends string,) ? S : never" type=T extends (infer S extends string,) ? Text.S : never
/// @definition.type symbol=Text source="type Text<T> = T extends (infer S extends string,) ? S : never" template=(T) value=T extends (infer S extends string,) ? Text.S : never
/// @type.symbol symbol=Text.T source=T type=T
/// @resolution.name source=T target=Text.T
/// @resolution.name source=S target=Text.S

declare const ok: Text<("a",)>;
/// @type.symbol symbol=ok source=ok type=Text<("a",)>
/// @resolution.pattern source=ok kind=binding target=ok
/// @resolution.name source=Text target=Text

declare const bad: Text<(1,)>;
/// @type.symbol symbol=bad source=bad type=Text<(1,)>
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Text target=Text
"#,
        r#"
"#,
    );
}

/// Join repeated covariant captures of one infer variable into a union.
#[test]
fn test_union_repeated_covariant_infer_captures() {
    let session = TestSession::single(
        r#"
type Both<T> = T extends { a: infer U, b: infer U } ? U : never;

declare const value: Both<{ a: string, b: int32 }>;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Both<T> = T extends { a: infer U; b: infer U } ? U : never;

declare const value: Both<{ a: string; b: int32 }>;

=== dir ===
type Both<T> = T extends { a: infer U, b: infer U } ? U : never;
/// @generic.template symbol=Both parameters=(T)
/// @type.symbol symbol=Both source="type Both<T> = T extends { a: infer U, b: infer U } ? U : never" type=T extends { a: infer U; b: infer U } ? Both.U : never
/// @definition.type symbol=Both source="type Both<T> = T extends { a: infer U, b: infer U } ? U : never" template=(T) value=T extends { a: infer U; b: infer U } ? Both.U : never
/// @type.symbol symbol=Both.T source=T type=T
/// @resolution.name source=T target=Both.T
/// @type.symbol symbol=Both.a source="a: infer U" type=infer U
/// @type.symbol symbol=Both.b source="b: infer U" type=infer U
/// @resolution.name source=U target=Both.U

declare const value: Both<{ a: string, b: int32 }>;
/// @type.symbol symbol=value source=value type=Both<{ a: string; b: int32 }>
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Both target=Both
/// @type.symbol symbol=a source="a: string" type=string
/// @type.symbol symbol=b source="b: int32" type=int32
"#,
        r#"
"#,
    );
}

/// Join repeated contravariant captures of one infer variable into an intersection.
#[test]
fn test_intersect_repeated_contravariant_infer_captures() {
    let session = TestSession::single(
        r#"
type Both<T> = T extends { f: (x: infer U) => void, g: (y: infer U) => void } ? U : "no";

declare const value: Both<{ f: (x: string) => void, g: (y: int32) => void }>;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Both<T> = T extends { f: (x: infer U) => void; g: (y: infer U) => void } ? U : "no";

declare const value: Both<{ f: (x: string) => void; g: (y: int32) => void }>;

=== dir ===
type Both<T> = T extends { f: (x: infer U) => void, g: (y: infer U) => void } ? U : "no";
/// @generic.template symbol=Both parameters=(T)
/// @type.symbol symbol=Both type=T extends { f: (infer U) => void; g: (infer U) => void } ? Both.U : "no"
/// @definition.type symbol=Both template=(T) value=T extends { f: (infer U) => void; g: (infer U) => void } ? Both.U : "no"
/// @type.symbol symbol=Both.T source=T type=T
/// @resolution.name source=T target=Both.T
/// @type.symbol symbol=Both.f source="f: (x: infer U) => void" type=(infer U) => void
/// @type.symbol symbol=Both.x source="x: infer U" type=infer U
/// @type.symbol symbol=Both.g source="g: (y: infer U) => void" type=(infer U) => void
/// @type.symbol symbol=Both.y source="y: infer U" type=infer U
/// @resolution.name source=U target=Both.U

declare const value: Both<{ f: (x: string) => void, g: (y: int32) => void }>;
/// @type.symbol symbol=value source=value type=Both<{ f: (string) => void; g: (int32) => void }>
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Both target=Both
/// @type.symbol symbol=f source="f: (x: string) => void" type=(string) => void
/// @type.symbol symbol=x source="x: string" type=string
/// @type.symbol symbol=g source="g: (y: int32) => void" type=(int32) => void
/// @type.symbol symbol=y source="y: int32" type=int32
"#,
        r#"
"#,
    );
}

/// Prefer the covariant capture when one infer variable is captured both ways.
#[test]
fn test_prefer_the_covariant_capture_of_one_infer_binder() {
    let session = TestSession::single(
        r#"
type Mixed<T> = T extends { a: infer U, f: (x: infer U) => void } ? U : never;

declare const narrow: Mixed<{ a: "a", f: (x: string) => void }>;
declare const wide: Mixed<{ a: int32, f: (x: string) => void }>;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Mixed<T> = T extends { a: infer U; f: (x: infer U) => void } ? U : never;

declare const narrow: Mixed<{ a: "a"; f: (x: string) => void }>;
declare const wide: Mixed<{ a: int32; f: (x: string) => void }>;

=== dir ===
type Mixed<T> = T extends { a: infer U, f: (x: infer U) => void } ? U : never;
/// @generic.template symbol=Mixed parameters=(T)
/// @type.symbol symbol=Mixed source="type Mixed<T> = T extends { a: infer U, f: (x: infer U) => void } ? U : never" type=T extends { a: infer U; f: (infer U) => void } ? Mixed.U : never
/// @definition.type symbol=Mixed source="type Mixed<T> = T extends { a: infer U, f: (x: infer U) => void } ? U : never" template=(T) value=T extends { a: infer U; f: (infer U) => void } ? Mixed.U : never
/// @type.symbol symbol=Mixed.T source=T type=T
/// @resolution.name source=T target=Mixed.T
/// @type.symbol symbol=Mixed.a source="a: infer U" type=infer U
/// @type.symbol symbol=Mixed.f source="f: (x: infer U) => void" type=(infer U) => void
/// @type.symbol symbol=Mixed.x source="x: infer U" type=infer U
/// @resolution.name source=U target=Mixed.U

declare const narrow: Mixed<{ a: "a", f: (x: string) => void }>;
/// @type.symbol symbol=narrow source=narrow type=Mixed<{ a: "a"; f: (string) => void }>
/// @resolution.pattern source=narrow kind=binding target=narrow
/// @resolution.name source=Mixed target=Mixed
/// @type.symbol symbol=a#1 source="a: \"a\"" type="a"
/// @type.symbol symbol=f#1 source="f: (x: string) => void" type=(string) => void
/// @type.symbol symbol=x#1 source="x: string" type=string

declare const wide: Mixed<{ a: int32, f: (x: string) => void }>;
/// @type.symbol symbol=wide source=wide type=Mixed<{ a: int32; f: (string) => void }>
/// @resolution.pattern source=wide kind=binding target=wide
/// @resolution.name source=Mixed target=Mixed
/// @type.symbol symbol=a#2 source="a: int32" type=int32
/// @type.symbol symbol=f#2 source="f: (x: string) => void" type=(string) => void
/// @type.symbol symbol=x#2 source="x: string" type=string
"#,
        r#"
"#,
    );
}

/// Bind one infer variable per conditional when conditionals nest.
#[test]
fn test_bind_one_infer_variable_per_nested_conditional() {
    let session = TestSession::single(
        r#"
type Deep<T> = T extends { value: infer V } ? (V extends (infer E, infer F) ? E : V) : never;

declare const element: Deep<{ value: (int32, string) }>;
declare const direct: Deep<{ value: string }>;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Deep<T> = T extends { value: infer V } ? (V extends (infer E, infer F) ? E : V) : never;

declare const element: Deep<{ value: (int32, string) }>;
declare const direct: Deep<{ value: string }>;

=== dir ===
type Deep<T> = T extends { value: infer V } ? (V extends (infer E, infer F) ? E : V) : never;
/// @generic.template symbol=Deep parameters=(T)
/// @type.symbol symbol=Deep type=T extends { value: infer V } ? Deep.V extends (infer E, infer F) ? Deep.E : Deep.V : never
/// @definition.type symbol=Deep template=(T) value=T extends { value: infer V } ? Deep.V extends (infer E, infer F) ? Deep.E : Deep.V : never
/// @type.symbol symbol=Deep.T source=T type=T
/// @resolution.name source=T target=Deep.T
/// @type.symbol symbol=Deep.value source="value: infer V" type=infer V
/// @resolution.name source=V target=Deep.V
/// @resolution.name source=E target=Deep.E
/// @resolution.name source=V target=Deep.V

declare const element: Deep<{ value: (int32, string) }>;
/// @type.symbol symbol=element source=element type=Deep<{ value: (int32, string) }>
/// @resolution.pattern source=element kind=binding target=element
/// @resolution.name source=Deep target=Deep
/// @type.symbol symbol=value#1 source="value: (int32, string)" type=(int32, string)

declare const direct: Deep<{ value: string }>;
/// @type.symbol symbol=direct source=direct type=Deep<{ value: string }>
/// @resolution.pattern source=direct kind=binding target=direct
/// @resolution.name source=Deep target=Deep
/// @type.symbol symbol=value#2 source="value: string" type=string
"#,
        r#"
"#,
    );
}

/// Assign a deferred conditional result to the union of both of its branches.
#[test]
fn test_assign_a_deferred_conditional_result_to_both_branches() {
    let session = TestSession::single(
        r#"
declare function pick<T>(value: T): T extends string ? 1 : 0;

function choose<T>(value: T): 1 | 0 {
    return pick(value);
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
declare function pick<T>(value: T): T extends string ? 1 : 0;

function choose<T>(value: T): 1 | 0 {
    return pick<T>(value);
}

=== dir ===
declare function pick<T>(value: T): T extends string ? 1 : 0;
/// @generic.template symbol=pick parameters=(T#1)
/// @type.symbol symbol=pick source="declare function pick<T>(value: T): T extends string ? 1 : 0" type=<T#1>(T#1) => T#1 extends string ? 1 : 0
/// @type.symbol symbol=pick.T source=T type=T#1
/// @resolution.name source=T target=pick.T
/// @resolution.name source=T target=pick.T

function choose<T>(value: T): 1 | 0 {
/// @generic.template symbol=choose parameters=(T#2)
/// @type.symbol symbol=choose type=<T#2>(T#2) => 1 | 0
/// @type.symbol symbol=choose.T source=T type=T#2
/// @type.symbol symbol=choose.value source="value: T" type=T#2
/// @resolution.name source=T target=choose.T

    return pick(value);
    /// @resolution.name source=pick target=pick
    /// @resolution.call source=pick(value) parameters=(T#2) arguments=(provided(value) as T#2) return=T#2 extends string ? 1 : 0 kind=symbol target=pick instance=pick<T#2>
    /// @generic.instantiation id=pick<T#2> template=pick arguments=(T#2) owner=choose
    /// @resolution.name source=value target=choose.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=choose.value

}
"#,
        r#"
"#,
    );
}

/// Reject an infer binder that sits outside a conditional extends clause.
#[test]
fn test_reject_infer_outside_a_conditional_extends_clause() {
    let session = TestSession::single(
        r#"
type Loose<T> = infer U;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Loose<T> = infer U;

=== dir ===
type Loose<T> = infer U;
/// @generic.template symbol=Loose parameters=(T)
/// @type.symbol symbol=Loose source="type Loose<T> = infer U" type=<error>
/// @definition.type symbol=Loose source="type Loose<T> = infer U" template=(T) value=<error>
/// @type.symbol symbol=Loose.T source=T type=T
"#,
        r#"
/// @diagnostic.error id=infer-outside-conditional message="'infer' declarations are only permitted in the 'extends' clause of a conditional type"
/// @diagnostic.label line=2 column=23 span="U" line_source="type Loose<T> = infer U;"
"#,
    );
}

/// Evaluate a conditional that tails into itself in place instead of nesting.
#[test]
fn test_evaluate_a_tail_recursive_conditional_in_place() {
    let session = TestSession::single(
        r#"
type Strip<S> = S extends `x${infer R}` ? Strip<R> : S;

declare const stripped: Strip<"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxy">;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Strip<S> = S extends `x${infer R}` ? Strip<R> : S;

declare const stripped: Strip<"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxy">;

=== dir ===
type Strip<S> = S extends `x${infer R}` ? Strip<R> : S;
/// @generic.template symbol=Strip parameters=(S)
/// @type.symbol symbol=Strip source="type Strip<S> = S extends `x${infer R}` ? Strip<R> : S" type=S extends `x${infer R}` ? Strip<Strip.R> : S
/// @definition.type symbol=Strip source="type Strip<S> = S extends `x${infer R}` ? Strip<R> : S" template=(S) value=S extends `x${infer R}` ? Strip<Strip.R> : S
/// @type.symbol symbol=Strip.S source=S type=S
/// @resolution.name source=S target=Strip.S
/// @resolution.name source=Strip target=Strip
/// @resolution.name source=R target=Strip.R
/// @resolution.name source=S target=Strip.S

declare const stripped: Strip<"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxy">;
/// @type.symbol symbol=stripped source=stripped type=Strip<"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxy">
/// @resolution.pattern source=stripped kind=binding target=stripped
/// @resolution.name source=Strip target=Strip
"#,
        r#"
"#,
    );
}

/// Report an endlessly recursive conditional alias instead of expanding it forever.
#[test]
fn test_reject_an_endlessly_recursive_conditional_alias() {
    let session = TestSession::single(
        r#"
type Grow<T> = T extends unknown ? Grow<(T,)> : never;

declare const value: Grow<int32>;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Grow<T> = T extends unknown ? Grow<(T,)> : never;

declare const value: Grow<int32>;

=== dir ===
type Grow<T> = T extends unknown ? Grow<(T,)> : never;
/// @generic.template symbol=Grow parameters=(T)
/// @type.symbol symbol=Grow source="type Grow<T> = T extends unknown ? Grow<(T,)> : never" type=T extends unknown ? Grow<(T,)> : never
/// @definition.type symbol=Grow source="type Grow<T> = T extends unknown ? Grow<(T,)> : never" template=(T) value=T extends unknown ? Grow<(T,)> : never
/// @type.symbol symbol=Grow.T source=T type=T
/// @resolution.name source=T target=Grow.T
/// @resolution.name source=Grow target=Grow
/// @resolution.name source=T target=Grow.T

declare const value: Grow<int32>;
/// @type.symbol symbol=value source=value type=Grow<int32>
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Grow target=Grow
"#,
        r#"
/// @diagnostic.error id=excessive-type-instantiation message="type instantiation is excessively deep and possibly infinite"
/// @diagnostic.label line=2 column=6 span="Grow" line_source="type Grow<T> = T extends unknown ? Grow<(T,)> : never;"
"#,
    );
}

/// Index the check type under the structure its true branch assumes.
#[test]
fn test_index_the_check_type_refined_by_the_true_branch() {
    let session = TestSession::single(
        r#"
type Value<T> = T extends { value: string } ? T["value"] : never;

declare const text: Value<{ value: "ready" }>;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Value<T> = T extends { value: string } ? T["value"] : never;

declare const text: Value<{ value: "ready" }>;

=== dir ===
type Value<T> = T extends { value: string } ? T["value"] : never;
/// @generic.template symbol=Value parameters=(T)
/// @type.symbol symbol=Value source="type Value<T> = T extends { value: string } ? T[\"value\"] : never" type=T extends { value: string } ? T["value"] : never
/// @definition.type symbol=Value source="type Value<T> = T extends { value: string } ? T[\"value\"] : never" template=(T) value=T extends { value: string } ? T["value"] : never
/// @type.symbol symbol=Value.T source=T type=T
/// @resolution.name source=T target=Value.T
/// @type.symbol symbol=Value.value source="value: string" type=string
/// @resolution.name source=T target=Value.T

declare const text: Value<{ value: "ready" }>;
/// @type.symbol symbol=text source=text type=Value<{ value: "ready" }>
/// @resolution.pattern source=text kind=binding target=text
/// @resolution.name source=Value target=Value
/// @type.symbol symbol=value source="value: \"ready\"" type="ready"
"#,
        r#"
"#,
    );
}
