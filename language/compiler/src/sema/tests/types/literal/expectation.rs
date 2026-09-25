use crate::tests::{DirRows, TestSession};

/// A literal expectation survives ternaries, returns, and block tails.
#[test]
fn test_keep_a_literal_expectation_through_a_block_tail() {
    let session = TestSession::single(
        r#"
type Mode = "fast" | "slow";

declare const quick: boolean;

const ternary: Mode = quick ? "fast" : "slow";
const closure: () => Mode = () => {
    if (quick) {
        return "fast";
    }
    return "slow";
};
const tail: () => Mode = () => "fast";
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Mode = "fast" | "slow";

declare const quick: boolean;

const ternary: Mode = quick ? "fast" : "slow";
const closure: () => Mode = (): Mode => {
    if (quick) {
        return "fast";
    }
    return "slow";
};
const tail: () => Mode = (): Mode => "fast";

=== dir ===
type Mode = "fast" | "slow";
/// @type.symbol symbol=Mode source="type Mode = \"fast\" | \"slow\"" type="fast" | "slow"
/// @definition.type symbol=Mode source="type Mode = \"fast\" | \"slow\"" value="fast" | "slow"

declare const quick: boolean;
/// @type.symbol symbol=quick source=quick type=boolean
/// @resolution.pattern source=quick kind=binding target=quick

const ternary: Mode = quick ? "fast" : "slow";
/// @type.symbol symbol=ternary source=ternary type=Mode
/// @resolution.pattern source=ternary kind=binding target=ternary
/// @resolution.name source=Mode target=Mode
/// @resolution.name source=quick target=quick
/// @resolution.place source=quick placement="local" lifetime="static" access="immutable"
/// @resolution.access source=quick root=quick

const closure: () => Mode = () => {
/// @type.symbol symbol=closure source=closure type=() => Mode
/// @resolution.pattern source=closure kind=binding target=closure
/// @resolution.name source=Mode target=Mode
/// @type.symbol symbol=symbol4 type=Function<(), Mode, "readonly">

    if (quick) {
    /// @resolution.name source=quick target=quick
    /// @resolution.place source=quick placement="local" lifetime="static" access="immutable"
    /// @resolution.access source=quick root=quick

        return "fast";
    }
    return "slow";
};
const tail: () => Mode = () => "fast";
/// @type.symbol symbol=tail source=tail type=() => Mode
/// @resolution.pattern source=tail kind=binding target=tail
/// @resolution.name source=Mode target=Mode
/// @type.symbol symbol=symbol6 source="() => \"fast\"" type=Function<(), Mode, "readonly">
"#,
        r#"
/// @diagnostic.error id=return-not-assignable message="type 'string' is not assignable to the declared result type 'Mode'"
/// @diagnostic.label line=9 column=16 span="\"fast\"" line_source="return \"fast\";"
/// @diagnostic.note message="'Mode' reduces to '\"fast\" | \"slow\"'"
/// @diagnostic.error id=return-not-assignable message="type 'string' is not assignable to the declared result type 'Mode'"
/// @diagnostic.label line=13 column=32 span="\"fast\"" line_source="const tail: () => Mode = () => \"fast\";"
/// @diagnostic.note message="'Mode' reduces to '\"fast\" | \"slow\"'"
"#,
    );
}

/// An integer binding solves from a statement that assigns it later.
#[test]
fn test_solve_an_integer_binding_from_a_later_statement() {
    let session = TestSession::single(
        r#"
function count(values: &readonly int32[]): int32 {
    let total = 0;
    for (const value of values) {
        total += value;
    }
    return total;
}

function fallback(result: Result<int32, string>): int32 {
    let value = 0;
    if (result.isOk()) {
        value = result.unwrap();
    }
    return value;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
function count<'a>(values: &'a readonly int32[]): int32 {
    let total: int32 = 0;
    for (const value of values) {
        total += value;
    }
    return total;
}

function fallback(result: Result<int32, string>): int32 {
    let value: int32 = 0;
    if (result.isOk<int32, string, "frame">()) {
        value = result.unwrap<int32, string>();
    }
    return value;
}

=== dir ===
function count(values: &readonly int32[]): int32 {
/// @generic.template symbol=count parameters=('a)
/// @type.symbol symbol=count type=<count.'a>(&count.'a readonly int32[]) => int32
/// @type.symbol symbol=count.values source="values: &readonly int32[]" type=&count.'a readonly int32[]

    let total = 0;
    /// @type.symbol symbol=count.total source=total type=int32
    /// @resolution.pattern source=total kind=binding target=count.total

    for (const value of values) {
    /// @resolution.iteration iterator="iterator#4(parameters=(), arguments=(), return=ArrayValueIterator<int32, count.'a>, regions=(count.'a))" next="next#2(parameters=(), arguments=(), return=IteratorResult<int32, void>, regions=(count.'a, \"frame\" & \"local\"))"
    /// @generic.instantiation id="iterator#4<int32, count.'a>" template=iterator#4 arguments=(int32, count.'a)
    /// @generic.instantiation id="next#2<int32, count.'a, \"frame\" & \"local\">" template=next#2 arguments=(int32, count.'a, "frame" & "local")
    /// @type.symbol symbol=count.value source=value type=int32
    /// @resolution.pattern source=value kind=binding target=count.value
    /// @resolution.name source=values target=count.values
    /// @resolution.place source=values placement=count.'a lifetime=count.'a access="readonly"
    /// @resolution.access source=values root=count.values

        total += value;
        /// @resolution.name source=total target=count.total
        /// @resolution.operator source="total += value" type=int32 operator="+" kind=builtin operands=[total as int32 families=(integer), value as int32 families=(integer)]
        /// @resolution.pattern.assign source=total kind=place
        /// @resolution.place source=total placement="local" lifetime="frame" access="exclusive"
        /// @resolution.assignment source=total read=binding(count.total) write=binding(count.total) type=int32
        /// @resolution.access source=total root=count.total
        /// @resolution.name source=value target=count.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="immutable"
        /// @resolution.access source=value root=count.value

    }
    return total;
    /// @resolution.name source=total target=count.total
    /// @resolution.place source=total placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=total root=count.total

}

function fallback(result: Result<int32, string>): int32 {
/// @type.symbol symbol=fallback type=(Result<int32, string>) => int32
/// @type.symbol symbol=fallback.result source="result: Result<int32, string>" type=Result<int32, string>
/// @resolution.name source=Result target=Result

    let value = 0;
    /// @type.symbol symbol=fallback.value source=value type=int32
    /// @resolution.pattern source=value kind=binding target=fallback.value

    if (result.isOk()) {
    /// @resolution.name source=result target=fallback.result
    /// @resolution.member source=result.isOk receiver=Result<int32, string> type=<isOk.'a>(this: &isOk.'a readonly Result<int32, string>) => boolean kind=symbol target_receiver=Result<int32, string> target=isOk
    /// @resolution.call source=result.isOk() parameters=() return=boolean regions=("frame" & "local") kind=symbol target=isOk receiver=Result<int32, string> adjustments=(borrow(&'frame readonly Result<int32, string>)) instance="Result<int32, string>.<extension#1>.isOk<\"frame\" & \"local\">"
    /// @resolution.place source=result placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=result root=fallback.result
    /// @generic.instantiation id="isOk<int32, string, \"frame\" & \"local\">" template=isOk arguments=(int32, string, "frame" & "local")
    /// @generic.instantiation id="isOk<int32, string>" template=isOk arguments=(int32, string)

        value = result.unwrap();
        /// @resolution.name source=value target=fallback.value
        /// @resolution.pattern.assign source=value kind=place
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=fallback.value
        /// @resolution.assignment source=value write=binding(fallback.value) type=int32
        /// @resolution.name source=result target=fallback.result
        /// @resolution.member source=result.unwrap receiver=Result<int32, string> type=(this: Result<int32, string>) => int32 kind=symbol target_receiver=Result<int32, string> target=unwrap
        /// @resolution.call source=result.unwrap() parameters=() return=int32 kind=symbol target=unwrap receiver=Result<int32, string> instance="Result<int32, string>.<extension#1>.unwrap"
        /// @resolution.place source=result placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=result root=fallback.result
        /// @generic.instantiation id="unwrap<int32, string>" template=unwrap arguments=(int32, string)

    }
    return value;
    /// @resolution.name source=value target=fallback.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=fallback.value

}
"#,
        r#"
"#,
    );
}

/// An integer binding solves from an expression use and from an operator.
#[test]
fn test_solve_an_integer_binding_from_an_expression_use_and_an_operator() {
    let session = TestSession::single(
        r#"
import { Result, Ok } from "tspp:error";

function value(result: Result<int32, string>): int32 {
    let fallback = 0;
    return if (let Ok { value } = result) {
        value
    } else {
        fallback
    };
}

function count(): int32 {
    let total = 0;
    for (let index = 0; index < 3; index++) {
        total = total - index;
    }
    return total;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { Ok, Result } from "tspp:error";

function value(result: Result<int32, string>): int32 {
    let fallback: int32 = 0;
    return if (let Ok { value } = result) {
        value
    } else {
        fallback
    };
}

function count(): int32 {
    let total: int32 = 0;
    for (let index: int32 = 0; index < 3; index++) {
        total = total - index;
    }
    return total;
}

=== dir ===
import { Result, Ok } from "tspp:error";

function value(result: Result<int32, string>): int32 {
/// @type.symbol symbol=value type=(Result<int32, string>) => int32
/// @type.symbol symbol=value.result source="result: Result<int32, string>" type=Result<int32, string>
/// @resolution.name source=Result target=Result

    let fallback = 0;
    /// @type.symbol symbol=value.fallback source=fallback type=int32
    /// @resolution.pattern source=fallback kind=binding target=value.fallback

    return if (let Ok { value } = result) {
    /// @resolution.name source=Ok target=Ok
    /// @resolution.pattern source="Ok { value }" kind=nominal_object adjustments=(newtype.payload(Result, Ok<int32> | Err<string>), union.payload(Ok<int32> | Err<string>, Ok<int32>, Ok<int32>)) target=Ok instance=Ok<int32> fields={ Ok.value }
    /// @generic.instantiation id="Result<int32, string>" template=Result arguments=(int32, string)
    /// @generic.instantiation id=Ok<int32> template=Ok arguments=(int32)
    /// @type.symbol symbol=value.value source=value type=int32
    /// @resolution.name source=result target=value.result
    /// @resolution.access source=result root=value.result

        value
        /// @resolution.name source=value target=value.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=value.value

    } else {
        fallback
        /// @resolution.name source=fallback target=value.fallback
        /// @resolution.place source=fallback placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=fallback root=value.fallback

    };
}

function count(): int32 {
/// @type.symbol symbol=count type=() => int32

    let total = 0;
    /// @type.symbol symbol=count.total source=total type=int32
    /// @resolution.pattern source=total kind=binding target=count.total

    for (let index = 0; index < 3; index++) {
    /// @type.symbol symbol=count.index source=index type=int32
    /// @resolution.pattern source=index kind=binding target=count.index
    /// @resolution.name source=index target=count.index
    /// @resolution.operator source="index < 3" type=boolean operator="<" kind=builtin operands=[index as int32 families=(integer), 3 as int32 families=(integer)]
    /// @resolution.place source=index placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=index root=count.index
    /// @resolution.name source=index target=count.index
    /// @resolution.assignment source=index read=binding(count.index) write=binding(count.index) type=int32
    /// @resolution.access source=index root=count.index
    /// @resolution.operator source=index++ type=int32 operator="++" kind=builtin operands=[index as int32 families=(integer)]

        total = total - index;
        /// @resolution.name source=total target=count.total
        /// @resolution.pattern.assign source=total kind=place
        /// @resolution.place source=total placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=total root=count.total
        /// @resolution.assignment source=total write=binding(count.total) type=int32
        /// @resolution.name source=total target=count.total
        /// @resolution.operator source="total - index" type=int32 operator="-" kind=builtin operands=[total as int32 families=(integer), index as int32 families=(integer)]
        /// @resolution.place source=total placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=total root=count.total
        /// @resolution.name source=index target=count.index
        /// @resolution.place source=index placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=index root=count.index

    }
    return total;
    /// @resolution.name source=total target=count.total
    /// @resolution.place source=total placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=total root=count.total

}
"#,
        r#"

"#,
    );
}

/// A match arm block tail types at the expected result type.
#[test]
fn test_type_a_match_arm_block_tail_at_the_expected_representation() {
    let session = TestSession::single(
        r#"
import { Result } from "tspp:error";

declare function observe(): void;

function value(result: Result<int32, string>): int32 {
    return match (result) {
        Ok { value } => value
        Err { error: _ } => {
            observe();
            0
        }
    };
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { Result } from "tspp:error";

declare function observe(): void;

function value(result: Result<int32, string>): int32 {
    return match (result) {
        Ok { value } => value
        Err { error: _ } => {
            observe();
            0
        }
    };
}

=== dir ===
import { Result } from "tspp:error";

declare function observe(): void;
/// @type.symbol symbol=observe source="declare function observe(): void" type=() => void

function value(result: Result<int32, string>): int32 {
/// @type.symbol symbol=value type=(Result<int32, string>) => int32
/// @type.symbol symbol=value.result source="result: Result<int32, string>" type=Result<int32, string>
/// @resolution.name source=Result target=Result

    return match (result) {
    /// @resolution.coverage exhaustive=true disjoint=true
    /// @resolution.name source=result target=value.result
    /// @resolution.place source=result placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=result root=value.result

        Ok { value } => value
        /// @resolution.name source=Ok target=Ok
        /// @resolution.pattern source="Ok { value }" kind=nominal_object adjustments=(newtype.payload(Result, Ok<int32> | Err<string>), union.payload(Ok<int32> | Err<string>, Ok<int32>, Ok<int32>)) target=Ok instance=Ok<int32> fields={ Ok.value }
        /// @generic.instantiation id="Result<int32, string>" template=Result arguments=(int32, string)
        /// @generic.instantiation id=Ok<int32> template=Ok arguments=(int32)
        /// @type.symbol symbol=value.value source=value type=int32
        /// @resolution.name source=value target=value.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="immutable"
        /// @resolution.access source=value root=value.value

        Err { error: _ } => {
        /// @resolution.name source=Err target=Err
        /// @resolution.pattern source="Err { error: _ }" kind=nominal_object adjustments=(newtype.payload(Result, Ok<int32> | Err<string>), union.payload(Ok<int32> | Err<string>, Err<string>, Err<string>)) target=Err instance=Err<string> fields={ Err.error: _ }
        /// @generic.instantiation id=Err<string> template=Err arguments=(string)
        /// @resolution.pattern source=_ kind=wildcard

            observe();
            /// @resolution.name source=observe target=observe
            /// @resolution.call source=observe() parameters=() return=void kind=symbol target=observe

            0
        }
    };
}
"#,
        r#"
"#,
    );
}

/// A compound assignment keeps the literal union a conditional produces.
#[test]
fn test_keep_a_conditional_literal_union_under_a_compound_assignment() {
    let session = TestSession::single(
        r#"
function sequence(depth: isize): string {
    let output = "";
    for (const index of 0..depth) {
        output += index == 0 ? "a" : "b";
    }
    return output;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
function sequence(depth: isize): string {
    let output: string = "";
    for (const index of 0..depth) {
        (output += index == 0 ? "a" : "b") as string;
    }
    return output;
}

=== dir ===
function sequence(depth: isize): string {
/// @type.symbol symbol=sequence type=(isize) => string
/// @type.symbol symbol=sequence.depth source="depth: isize" type=isize

    let output = "";
    /// @type.symbol symbol=sequence.output source=output type=string
    /// @resolution.pattern source=output kind=binding target=sequence.output

    for (const index of 0..depth) {
    /// @resolution.iteration iterator="iterator#1(parameters=(), arguments=(), return=RangeIterator<isize>)" next="next(parameters=(), arguments=(), return=IteratorResult<isize, void>, regions=(\"frame\" & \"local\"))"
    /// @generic.instantiation id="next<isize, \"frame\" & \"local\">" template=next arguments=(isize, "frame" & "local")
    /// @generic.instantiation id=iterator#1<isize> template=iterator#1 arguments=(isize)
    /// @type.symbol symbol=sequence.index source=index type=isize
    /// @resolution.pattern source=index kind=binding target=sequence.index
    /// @resolution.name source=depth target=sequence.depth
    /// @resolution.place source=depth placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=depth root=sequence.depth

        output += index == 0 ? "a" : "b";
        /// @resolution.name source=output target=sequence.output
        /// @resolution.operator source="output += index == 0 ? \"a\" : \"b\"" type=^string operator="+" kind=call parameters=(string) arguments=(provided(index == 0 ? "a" : "b") as string) return=^string regions=("managed" & "local") kind=symbol target=add receiver=string adjustments=(borrow(&'managed readonly string)) instance="string.<extension#2>.add<\"managed\" & \"local\">"
        /// @resolution.pattern.assign source=output kind=place
        /// @resolution.place source=output placement="local" lifetime="frame" access="exclusive"
        /// @resolution.assignment source=output read=binding(sequence.output) write=binding(sequence.output) type=string
        /// @resolution.access source=output root=sequence.output
        /// @generic.instantiation id="add<\"managed\" & \"local\">" template=add arguments=("managed" & "local")
        /// @resolution.name source=index target=sequence.index
        /// @resolution.operator source="index == 0" type=boolean operator="==" kind=builtin operands=[index as isize families=(integer), 0 as isize families=(integer)]
        /// @resolution.place source=index placement="local" lifetime="frame" access="immutable"
        /// @resolution.access source=index root=sequence.index

    }
    return output;
    /// @resolution.name source=output target=sequence.output
    /// @resolution.place source=output placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=output root=sequence.output

}
"#,
        r#"

"#,
    );
}

/// A closure block tail types at the expected return type.
#[test]
fn test_type_a_closure_block_tail_at_the_expected_return() {
    let session = TestSession::single(
        r#"
declare function observe(): void;
declare function take(compute: () => int32): int32;

const taken = take(() => {
    observe();
    0
});
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
declare function observe(): void;
declare function take(compute: () => int32): int32;

const taken: int32 = take((): int32 => {
    observe();
    0
});

=== dir ===
declare function observe(): void;
/// @type.symbol symbol=observe source="declare function observe(): void" type=() => void

declare function take(compute: () => int32): int32;
/// @type.symbol symbol=take source="declare function take(compute: () => int32): int32" type=(() => int32) => int32

const taken = take(() => {
/// @type.symbol symbol=taken source=taken type=int32
/// @resolution.pattern source=taken kind=binding target=taken
/// @resolution.name source=take target=take
/// @resolution.call parameters=(() => int32) arguments=(provided(argument) as () => int32) return=int32 kind=symbol target=take
/// @type.symbol symbol=symbol4 type=Function<(), int32, "readonly">

    observe();
    /// @resolution.name source=observe target=observe
    /// @resolution.call source=observe() parameters=() return=void kind=symbol target=observe

    0
});
"#,
        r#"

"#,
    );
}

/// Integer literals default to int64 across operators, members, and arrays.
#[test]
fn test_default_integer_literals_to_int64_across_operators_and_members() {
    let session = TestSession::single(
        r#"
function settle(): void {
    const sum = 1 + 1;
    const text = (1).toString();
    const mixed = [1, 2.5];
    let counter = 1;
    counter += 2;
    const compared = counter < 10;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
function settle(): void {
    const sum: 2 = 1 + 1;
    const text: ^string = (1).toString<int64>();
    const mixed: float64[] = [1, 2.5];
    let counter: int64 = 1;
    counter += 2;
    const compared: boolean = counter < 10;
}

=== dir ===
function settle(): void {
/// @type.symbol symbol=settle type=() => void

    const sum = 1 + 1;
    /// @type.symbol symbol=settle.sum source=sum type=2
    /// @resolution.pattern source=sum kind=binding target=settle.sum
    /// @resolution.operator source="1 + 1" type=2 operator="+" kind=builtin operands=[1 as 1 families=(integer), 1 as 1 families=(integer)]

    const text = (1).toString();
    /// @type.symbol symbol=settle.text source=text type=^string
    /// @resolution.pattern source=text kind=binding target=settle.text
    /// @resolution.member source=(1).toString receiver=1 type=(this: int64, float64 | undefined?) => ^string kind=symbol target_receiver=1 target=toString
    /// @resolution.call source=(1).toString() parameters=(float64 | undefined) arguments=(omitted as float64 | undefined) return=^string kind=symbol target=toString receiver=1 instance=int64.<extension#1>.toString
    /// @generic.instantiation id=toString<int64> template=toString arguments=(int64)

    const mixed = [1, 2.5];
    /// @type.symbol symbol=settle.mixed source=mixed type=float64[]
    /// @resolution.pattern source=mixed kind=binding target=settle.mixed
    /// @resolution.call source=[1, 2.5] parameters=(^Slice<float64>) arguments=(rest(provided(1) as float64, provided(2.5) as float64) as float64) return=float64[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<float64>
    /// @generic.instantiation id=arrayFromOwnedSlice<float64> template=arrayFromOwnedSlice arguments=(float64)

    let counter = 1;
    /// @type.symbol symbol=settle.counter source=counter type=int64
    /// @resolution.pattern source=counter kind=binding target=settle.counter

    counter += 2;
    /// @resolution.name source=counter target=settle.counter
    /// @resolution.operator source="counter += 2" type=int64 operator="+" kind=builtin operands=[counter as int64 families=(integer), 2 as int64 families=(integer)]
    /// @resolution.pattern.assign source=counter kind=place
    /// @resolution.place source=counter placement="local" lifetime="frame" access="exclusive"
    /// @resolution.assignment source=counter read=binding(settle.counter) write=binding(settle.counter) type=int64
    /// @resolution.access source=counter root=settle.counter

    const compared = counter < 10;
    /// @type.symbol symbol=settle.compared source=compared type=boolean
    /// @resolution.pattern source=compared kind=binding target=settle.compared
    /// @resolution.name source=counter target=settle.counter
    /// @resolution.operator source="counter < 10" type=boolean operator="<" kind=builtin operands=[counter as int64 families=(integer), 10 as int64 families=(integer)]
    /// @resolution.place source=counter placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=counter root=settle.counter

}
"#,
        r#"

"#,
    );
}
