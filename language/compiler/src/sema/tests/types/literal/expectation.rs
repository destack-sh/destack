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
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Mode = "fast" | "slow";

declare const quick: boolean;

const ternary: "fast" | "slow" = quick ? ("fast" as "fast" | "slow") : ("slow" as "fast" | "slow");
const closure: () => Mode = (): Mode => {
    if (quick) {
        return "fast" as Mode;
    }
    return "slow" as Mode;
};
const tail: () => Mode = (): Mode => "fast" as Mode;

=== dir ===
type Mode = "fast" | "slow";
/// @type.symbol symbol=Mode source="type Mode = \"fast\" | \"slow\"" type="fast" | "slow"
/// @definition.type symbol=Mode source="type Mode = \"fast\" | \"slow\"" value="fast" | "slow"

declare const quick: boolean;
/// @type.symbol symbol=quick source=quick type=boolean
/// @resolution.pattern source=quick kind=binding target=quick

const ternary: Mode = quick ? "fast" : "slow";
/// @type.symbol symbol=ternary source=ternary type="fast" | "slow"
/// @resolution.pattern source=ternary kind=binding target=ternary
/// @resolution.name source=Mode target=Mode
/// @resolution.name source=quick target=quick
/// @resolution.place source=quick placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=quick root=quick

const closure: () => Mode = () => {
/// @type.symbol symbol=closure source=closure type=Function<(), Mode>
/// @resolution.pattern source=closure kind=binding target=closure
/// @resolution.name source=Mode target=Mode
/// @type.symbol symbol=symbol4 type=Function<(), Mode>

    if (quick) {
    /// @resolution.name source=quick target=quick
    /// @resolution.place source=quick placement="constant" lifetime="static" access="readonly"
    /// @resolution.access source=quick root=quick

        return "fast";
    }
    return "slow";
};
const tail: () => Mode = () => "fast";
/// @type.symbol symbol=tail source=tail type=Function<(), Mode>
/// @resolution.pattern source=tail kind=binding target=tail
/// @resolution.name source=Mode target=Mode
/// @type.symbol symbol=symbol6 source="() => \"fast\"" type=Function<(), Mode>
"#,
        r#"

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
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function count<'a, P1: Place>(values: &'a readonly int32[]): int32 {
    let total: int32 = 0;
    for (const value of values) {
        total += value;
    }
    return total;
}

function fallback(result: Result<int32, string>): int32 {
    let value: int32 = 0;
    if (result.isOk<int32, string, "local">()) {
        value = result.unwrap<int32, string>();
    }
    return value;
}

=== dir ===
function count(values: &readonly int32[]): int32 {
/// @generic.template symbol=count parameters=('a, P1: Place)
/// @type.symbol symbol=count type=<count.'a, count.P1: Place>(&count.'a readonly int32[]) => int32
/// @type.symbol symbol=count.values source="values: &readonly int32[]" type=&count.'a readonly int32[]

    let total = 0;
    /// @type.symbol symbol=count.total source=total type=int32
    /// @resolution.pattern source=total kind=binding target=count.total

    for (const value of values) {
    /// @type.symbol symbol=count.value source=value type=int32
    /// @resolution.pattern source=value kind=binding target=count.value
    /// @resolution.name source=values target=count.values
    /// @resolution.access source=values root=count.values

        total += value;
        /// @resolution.name source=total target=count.total
        /// @resolution.operator source="total += value" type=int32 operator="+" kind=builtin operands=[total as int32 families=(integer), value as int32 families=(integer)]
        /// @resolution.pattern.assign source=total kind=place
        /// @resolution.place source=total placement="local" lifetime="frame" access="exclusive"
        /// @resolution.assignment source=total read=binding(count.total) write=binding(count.total) type=int32
        /// @resolution.access source=total root=count.total
        /// @resolution.name source=value target=count.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="readonly"
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
    /// @resolution.member source=result.isOk receiver=Result<int32, string> type=<isOk.'a, isOk.P1: Place>(this: Borrowed<Result<int32, string>, isOk.'a & isOk.P1, "readonly">) => boolean kind=symbol target_receiver=Result<int32, string> target=isOk
    /// @resolution.call source=result.isOk() parameters=() return=boolean kind=symbol target=isOk receiver=Result<int32, string> adjustments=(borrow(&'frame readonly Result<int32, string>)) instance="Result<int32, string>.<extension#1>.isOk<\"local\">"
    /// @resolution.place source=result placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=result root=fallback.result
    /// @generic.instantiation id="isOk<int32, string, \"local\">" template=isOk arguments=(int32, string, "local")
    /// @generic.instantiation id="isOk<int32, string>" template=isOk arguments=(int32, string)

        value = result.unwrap();
        /// @resolution.name source=value target=fallback.value
        /// @resolution.pattern.assign source=value kind=place
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
import { Result, Ok } from "destack:error";

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
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Ok, Result } from "destack:error";

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
import { Result, Ok } from "destack:error";

function value(result: Result<int32, string>): int32 {
/// @type.symbol symbol=value type=(Result<int32, string>) => int32
/// @type.symbol symbol=value.result source="result: Result<int32, string>" type=Result<int32, string>
/// @resolution.name source=Result target=Result

    let fallback = 0;
    /// @type.symbol symbol=value.fallback source=fallback type=int32
    /// @resolution.pattern source=fallback kind=binding target=value.fallback

    return if (let Ok { value } = result) {
    /// @resolution.name source=Ok target=Ok
    /// @resolution.pattern source="Ok { value }" kind=nominal_object target=Ok instance=Ok<int32> fields={ Ok.value }
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
import { Result } from "destack:error";

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
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Result } from "destack:error";

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
import { Result } from "destack:error";

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
        /// @resolution.pattern source="Ok { value }" kind=nominal_object target=Ok instance=Ok<int32> fields={ Ok.value }
        /// @generic.instantiation id=Ok<int32> template=Ok arguments=(int32)
        /// @type.symbol symbol=value.value source=value type=int32
        /// @resolution.name source=value target=value.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="readonly"
        /// @resolution.access source=value root=value.value

        Err { error: _ } => {
        /// @resolution.name source=Err target=Err
        /// @resolution.pattern source="Err { error: _ }" kind=nominal_object target=Err instance=Err<string> fields={ Err.error: _ }
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
        "main.ds",
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
    /// @type.symbol symbol=sequence.index source=index type=isize
    /// @resolution.pattern source=index kind=binding target=sequence.index
    /// @resolution.name source=depth target=sequence.depth
    /// @resolution.place source=depth placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=depth root=sequence.depth

        output += index == 0 ? "a" : "b";
        /// @resolution.name source=output target=sequence.output
        /// @resolution.operator source="output += index == 0 ? \"a\" : \"b\"" type=Owned<string> operator="+" kind=call parameters=(string) arguments=(provided(index == 0 ? "a" : "b") as string) return=Owned<string> kind=symbol target=add receiver=string adjustments=(borrow(&'frame readonly string)) instance="string.<extension#2>.add<\"local\">"
        /// @resolution.pattern.assign source=output kind=place
        /// @resolution.place source=output placement="local" lifetime="frame" access="exclusive"
        /// @resolution.assignment source=output read=binding(sequence.output) write=binding(sequence.output) type=string
        /// @resolution.access source=output root=sequence.output
        /// @generic.instantiation id="add<\"local\">" template=add arguments=("local")
        /// @resolution.name source=index target=sequence.index
        /// @resolution.operator source="index == 0" type=boolean operator="==" kind=builtin operands=[index as isize families=(integer), 0 as isize families=(integer)]
        /// @resolution.place source=index placement="local" lifetime="frame" access="readonly"
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
        "main.ds",
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
/// @type.symbol symbol=take source="declare function take(compute: () => int32): int32" type=(Function<(), int32>) => int32
/// @type.symbol symbol=take.compute source="compute: () => int32" type=Function<(), int32>

const taken = take(() => {
/// @type.symbol symbol=taken source=taken type=int32
/// @resolution.pattern source=taken kind=binding target=taken
/// @resolution.name source=take target=take
/// @resolution.call parameters=(Function<(), int32>) arguments=(provided(argument) as Function<(), int32>) return=int32 kind=symbol target=take
/// @type.symbol symbol=symbol4 type=Function<(), int32>

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
const sum = 1 + 1;
const text = (1).toString();
const mixed = [1, 2.5];
let counter = 1;
counter += 2;
const compared = counter < 10;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
const sum: 2 = 1 + 1;
const text: MaybeOwned<string> = (1).toString<"local">();
const mixed: float64[] = [1, 2.5];
let counter: int64 = 1;
counter += 2;
const compared: boolean = counter < 10;

=== dir ===
const sum = 1 + 1;
/// @type.symbol symbol=sum source=sum type=2
/// @resolution.pattern source=sum kind=binding target=sum
/// @resolution.operator source="1 + 1" type=2 operator="+" kind=builtin operands=[1 as 1 families=(integer), 1 as 1 families=(integer)]

const text = (1).toString();
/// @type.symbol symbol=text source=text type=MaybeOwned<string>
/// @resolution.pattern source=text kind=binding target=text
/// @resolution.member source=(1).toString receiver=1 type=<Number.toString.'a, Number.toString.P1: Place>(this: Borrowed<1, Number.toString.'a & Number.toString.P1, "readonly">, float64 | undefined?) => MaybeOwned<string> kind=symbol target_receiver=1 target=Number.toString
/// @resolution.call source=(1).toString() parameters=(float64 | undefined) arguments=(omitted as float64 | undefined) return=MaybeOwned<string> kind=symbol target=Number.toString receiver=1 adjustments=(borrow(&'frame readonly 1)) instance="Number.toString<\"local\">"
/// @generic.instantiation id="Number.toString<\"local\">" template=Number.toString arguments=("local")

const mixed = [1, 2.5];
/// @type.symbol symbol=mixed source=mixed type=float64[]
/// @resolution.pattern source=mixed kind=binding target=mixed
/// @resolution.call source=[1, 2.5] parameters=(&arrayFromSlice.'a readonly Slice<arrayFromSlice.T>) arguments=(rest(1, 2.5) as float64) return=float64[] kind=symbol target=arrayFromSlice instance=arrayFromSlice<float64>
/// @generic.instantiation id=arrayFromSlice<float64> template=arrayFromSlice arguments=(float64)

let counter = 1;
/// @type.symbol symbol=counter source=counter type=int64
/// @resolution.pattern source=counter kind=binding target=counter

counter += 2;
/// @resolution.name source=counter target=counter
/// @resolution.operator source="counter += 2" type=int64 operator="+" kind=builtin operands=[counter as int64 families=(integer), 2 as int64 families=(integer)]
/// @resolution.pattern.assign source=counter kind=place
/// @resolution.place source=counter placement="local" lifetime="static" access="exclusive"
/// @resolution.assignment source=counter read=binding(counter) write=binding(counter) type=int64
/// @resolution.access source=counter root=counter

const compared = counter < 10;
/// @type.symbol symbol=compared source=compared type=boolean
/// @resolution.pattern source=compared kind=binding target=compared
/// @resolution.name source=counter target=counter
/// @resolution.operator source="counter < 10" type=boolean operator="<" kind=builtin operands=[counter as int64 families=(integer), 10 as int64 families=(integer)]
/// @resolution.place source=counter placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=counter root=counter
"#,
        r#"

"#,
    );
}
