use crate::tests::{DirRows, TestSession};

#[test]
fn test_infer_a_callback_parameter_from_the_receiver_before_the_result() {
    let session = TestSession::single(
        r#"
import { Result, Ok } from "tspp:error";

declare const parsed: Result<int32, string>;

const doubled = parsed.andThen((value) => Result.ok(value * 2));
const wrapped = parsed.andThen((value) => Result(Ok { value: value * 2 }));
const named = parsed.map((value) => value + 1);
const unwrapped = parsed.andThen((value) => Ok { value });
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { Ok, Result } from "tspp:error";

declare const parsed: Result<int32, string>;

const doubled = parsed.andThen((value: int32) => Result.ok(value * 2));
const wrapped = parsed.andThen((value: int32) => Result(Ok<int32> { value: value * 2 }));
const named: Result<int32, string> = parsed.map<int32, string, int32>(
    (value: int32): int32 => value + 1,
);
const unwrapped = parsed.andThen((value: int32) => Ok<int32> { value });

=== dir ===
import { Result, Ok } from "tspp:error";

declare const parsed: Result<int32, string>;
/// @type.symbol symbol=parsed source=parsed type=Result<int32, string>
/// @resolution.pattern source=parsed kind=binding target=parsed
/// @resolution.name source=Result target=Result

const doubled = parsed.andThen((value) => Result.ok(value * 2));
/// @type.symbol symbol=doubled source=doubled type=Result<int32, string | <error>>
/// @resolution.pattern source=doubled kind=binding target=doubled
/// @resolution.name source=parsed target=parsed
/// @resolution.member source=parsed.andThen receiver=Result<int32, string> type=<andThen.U, andThen.F>(this: Result<int32, string>, (int32) => Result<andThen.U, andThen.F>) => Result<andThen.U, string | andThen.F> kind=symbol target_receiver=Result<int32, string> target=andThen
/// @resolution.call source="parsed.andThen((value) => Result.ok(value * 2))" parameters=((int32) => Result<int32, <error>>) arguments=(provided((value) => Result.ok(value * 2)) as (int32) => Result<int32, <error>>) return=Result<int32, string | <error>> kind=symbol target=andThen receiver=Result<int32, string> instance="Result<int32, string>.<extension#1>.andThen<int32, <error>>"
/// @resolution.place source=parsed placement="local" lifetime="static" access="immutable"
/// @resolution.access source=parsed root=parsed
/// @generic.instantiation id="andThen<int32, string>" template=andThen arguments=(int32, string)
/// @type.symbol symbol=symbol4 source=(value) => Result.ok(value * 2) type=Function<(int32,), Result<int32, <error>>, "readonly">
/// @type.symbol symbol=symbol4.value source=value type=int32
/// @resolution.name source=Result target=Result
/// @resolution.member source=Result.ok receiver=Result type=(T#1) => Result<T#1, E#1> kind=symbol target_receiver=Result target=ok#1
/// @resolution.call source="Result.ok(value * 2)" parameters=(int32) arguments=(provided(value * 2) as int32) return=Result<int32, <error>> kind=symbol target=ok#1 instance="Result<int32, <error>>.<extension#1>.ok#1"
/// @resolution.name source=value target=symbol4.value
/// @resolution.operator source="value * 2" type=int32 operator="*" kind=builtin operands=[value as int32 families=(integer), 2 as int32 families=(integer)]
/// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
/// @resolution.access source=value root=symbol4.value

const wrapped = parsed.andThen((value) => Result(Ok { value: value * 2 }));
/// @type.symbol symbol=wrapped source=wrapped type=Result<int32, string | <error>>
/// @resolution.pattern source=wrapped kind=binding target=wrapped
/// @resolution.name source=parsed target=parsed
/// @resolution.member source=parsed.andThen receiver=Result<int32, string> type=<andThen.U, andThen.F>(this: Result<int32, string>, (int32) => Result<andThen.U, andThen.F>) => Result<andThen.U, string | andThen.F> kind=symbol target_receiver=Result<int32, string> target=andThen
/// @resolution.call source="parsed.andThen((value) => Result(Ok { value: value * 2 }))" parameters=((int32) => Result<int32, <error>>) arguments=(provided((value) => Result(Ok { value: value * 2 })) as (int32) => Result<int32, <error>>) return=Result<int32, string | <error>> kind=symbol target=andThen receiver=Result<int32, string> instance="Result<int32, string>.<extension#1>.andThen<int32, <error>>"
/// @resolution.place source=parsed placement="local" lifetime="static" access="immutable"
/// @resolution.access source=parsed root=parsed
/// @type.symbol symbol=symbol7 source=(value) => Result(Ok { value: value * 2 }) type=Function<(int32,), Result<int32, <error>>, "readonly">
/// @type.symbol symbol=symbol7.value source=value type=int32
/// @resolution.name source=Result target=Result
/// @resolution.construct source="Result(Ok { value: value * 2 })" parameters=(Ok<int32>) arguments=(provided(Ok { value: value * 2 }) as Ok<int32>) return=Result<int32, <error>> kind=newtype target=Result backing=Ok<int32> instance="Result<int32, <error>>"
/// @resolution.name source=Ok target=Ok
/// @resolution.name source=value target=symbol7.value
/// @resolution.operator source="value * 2" type=int32 operator="*" kind=builtin operands=[value as int32 families=(integer), 2 as int32 families=(integer)]
/// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
/// @resolution.access source=value root=symbol7.value

const named = parsed.map((value) => value + 1);
/// @type.symbol symbol=named source=named type=Result<int32, string>
/// @resolution.pattern source=named kind=binding target=named
/// @resolution.name source=parsed target=parsed
/// @resolution.member source=parsed.map receiver=Result<int32, string> type=<map.U>(this: Result<int32, string>, (int32) => map.U) => Result<map.U, string> kind=symbol target_receiver=Result<int32, string> target=map
/// @resolution.call source="parsed.map((value) => value + 1)" parameters=((int32) => int32) arguments=(provided((value) => value + 1) as (int32) => int32) return=Result<int32, string> kind=symbol target=map receiver=Result<int32, string> instance="Result<int32, string>.<extension#1>.map<int32>"
/// @resolution.place source=parsed placement="local" lifetime="static" access="immutable"
/// @resolution.access source=parsed root=parsed
/// @generic.instantiation id="map<int32, string, int32>" template=map arguments=(int32, string, int32)
/// @generic.instantiation id="map<int32, string>" template=map arguments=(int32, string)
/// @type.symbol symbol=symbol10 source="(value) => value + 1" type=Function<(int32,), int32, "readonly">
/// @type.symbol symbol=symbol10.value source=value type=int32
/// @resolution.name source=value target=symbol10.value
/// @resolution.operator source="value + 1" type=int32 operator="+" kind=builtin operands=[value as int32 families=(integer), 1 as int32 families=(integer)]
/// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
/// @resolution.access source=value root=symbol10.value

const unwrapped = parsed.andThen((value) => Ok { value });
/// @type.symbol symbol=unwrapped source=unwrapped type=Result<<error>, string | <error>>
/// @resolution.pattern source=unwrapped kind=binding target=unwrapped
/// @resolution.name source=parsed target=parsed
/// @resolution.member source=parsed.andThen receiver=Result<int32, string> type=<andThen.U, andThen.F>(this: Result<int32, string>, (int32) => Result<andThen.U, andThen.F>) => Result<andThen.U, string | andThen.F> kind=symbol target_receiver=Result<int32, string> target=andThen
/// @resolution.call source="parsed.andThen((value) => Ok { value })" parameters=((int32) => Result<<error>, <error>>) arguments=(provided((value) => Ok { value }) as (int32) => Result<<error>, <error>>) return=Result<<error>, string | <error>> kind=symbol target=andThen receiver=Result<int32, string> instance="Result<int32, string>.<extension#1>.andThen<<error>, <error>>"
/// @resolution.place source=parsed placement="local" lifetime="static" access="immutable"
/// @resolution.access source=parsed root=parsed
/// @type.symbol symbol=symbol13 source="(value) => Ok { value }" type=Function<(int32,), Result<<error>, <error>>, "readonly">
/// @type.symbol symbol=symbol13.value source=value type=int32
/// @resolution.name source=Ok target=Ok
/// @resolution.name source=value target=symbol13.value
/// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
/// @resolution.access source=value root=symbol13.value
"#,
        r#"
/// @diagnostic.error id=cannot-infer-type message="cannot infer a type here"
/// @diagnostic.label line=6 column=32 span="(value) => Result.ok(value * 2)" line_source="const doubled = parsed.andThen((value) => Result.ok(value * 2));"
/// @diagnostic.help message="annotate the type explicitly"
/// @diagnostic.error id=cannot-infer-type message="cannot infer a type here"
/// @diagnostic.label line=7 column=32 span="(value) => Result(Ok { value: value * 2 })" line_source="const wrapped = parsed.andThen((value) => Result(Ok { value: value * 2 }));"
/// @diagnostic.help message="annotate the type explicitly"
/// @diagnostic.error id=cannot-infer-type message="cannot infer a type here"
/// @diagnostic.label line=9 column=34 span="(value) => Ok { value }" line_source="const unwrapped = parsed.andThen((value) => Ok { value });"
/// @diagnostic.help message="annotate the type explicitly"
"#,
    );
}

#[test]
fn test_infer_a_rest_pack_parameter_from_positional_closure_parameters() {
    let session = TestSession::single(
        r#"
declare function each<P: (...unknown[],) & Copy>(cases: P[]): (name: string, run: (...args: P) => void) => void;

each([(1, 2), (3, 4)])("adds", (left, right) => {
    const sum = left + right;
});

each([("a", 1)])("pairs", (text, count) => {
    const pair = (text, count);
});

each<(int32,)>([(1,)])("one", (value: int32) => {});

each([(1,)])("closed", (value: int32) => {});
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
declare function each<P: (...unknown[],) & Copy>(
    cases: P[],
): (name: string, run: (...args: P) => void) => void;

each<(int64, int64)>([(1, 2), (3, 4)])("adds", (left: int64, right: int64): void => {
    const sum: int64 = left + right;
});

each<(string, int64)>([("a", 1)])("pairs", (text: string, count: int64): void => {
    const pair: (string, int64) = (text, count);
});

each<(int32,)>([(1,)])("one", (value: int32): void => {});

each<(int32,)>([(1,)])("closed", (value: int32): void => {});

=== dir ===
declare function each<P: (...unknown[],) & Copy>(cases: P[]): (name: string, run: (...args: P) => void) => void;
/// @generic.template symbol=each parameters=(P: (...unknown[],) & Copy)
/// @type.symbol symbol=each type=<P: (...unknown[],) & Copy>(P[]) => (string, (...P) => void) => void
/// @type.symbol symbol=each.P source="P: (...unknown[],) & Copy" type=P
/// @resolution.name source=Copy target=Copy
/// @resolution.name source=P target=each.P
/// @type.symbol symbol=each.name source="name: string" type=string
/// @type.symbol symbol=each.run source="run: (...args: P) => void" type=(...P) => void
/// @type.symbol symbol=each.args source="...args: P" type=P
/// @resolution.name source=P target=each.P

each([(1, 2), (3, 4)])("adds", (left, right) => {
/// @resolution.name source=each target=each
/// @resolution.call parameters=(string, (...(int64, int64)) => void) arguments=(provided("adds") as string, provided(argument) as (...(int64, int64)) => void) return=void kind=expression target=expression
/// @resolution.call source="each([(1, 2), (3, 4)])" parameters=((int64, int64)[]) arguments=(provided([(1, 2), (3, 4)]) as (int64, int64)[]) return=(string, (...(int64, int64)) => void) => void kind=symbol target=each instance="each<(int64, int64)>"
/// @generic.instantiation id="each<(int64, int64)>" template=each arguments=((int64, int64))
/// @resolution.call source=[(1, 2), (3, 4)] parameters=(^Slice<(int64, int64)>) arguments=(rest(provided((1, 2)) as (int64, int64), provided((3, 4)) as (int64, int64)) as (int64, int64)) return=(int64, int64)[] kind=symbol target=arrayFromOwnedSlice instance="arrayFromOwnedSlice<(int64, int64)>"
/// @generic.instantiation id="arrayFromOwnedSlice<(int64, int64)>" template=arrayFromOwnedSlice arguments=((int64, int64))
/// @type.symbol symbol=symbol7 type=Function<(int64, int64), void, "readonly">
/// @type.symbol symbol=symbol7.left source=left type=int64
/// @type.symbol symbol=symbol7.right source=right type=int64

    const sum = left + right;
    /// @type.symbol symbol=symbol7.sum source=sum type=int64
    /// @resolution.pattern source=sum kind=binding target=symbol7.sum
    /// @resolution.name source=left target=symbol7.left
    /// @resolution.operator source="left + right" type=int64 operator="+" kind=builtin operands=[left as int64 families=(integer), right as int64 families=(integer)]
    /// @resolution.place source=left placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=left root=symbol7.left
    /// @resolution.name source=right target=symbol7.right
    /// @resolution.place source=right placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=right root=symbol7.right

});

each([("a", 1)])("pairs", (text, count) => {
/// @resolution.name source=each target=each
/// @resolution.call parameters=(string, (...(string, int64)) => void) arguments=(provided("pairs") as string, provided(argument) as (...(string, int64)) => void) return=void kind=expression target=expression
/// @resolution.call source="each([(\"a\", 1)])" parameters=((string, int64)[]) arguments=(provided([("a", 1)]) as (string, int64)[]) return=(string, (...(string, int64)) => void) => void kind=symbol target=each instance="each<(string, int64)>"
/// @generic.instantiation id="each<(string, int64)>" template=each arguments=((string, int64))
/// @resolution.call source=[("a", 1)] parameters=(^Slice<(string, int64)>) arguments=(rest(provided(("a", 1)) as (string, int64)) as (string, int64)) return=(string, int64)[] kind=symbol target=arrayFromOwnedSlice instance="arrayFromOwnedSlice<(string, int64)>"
/// @generic.instantiation id="arrayFromOwnedSlice<(string, int64)>" template=arrayFromOwnedSlice arguments=((string, int64))
/// @type.symbol symbol=symbol11 type=Function<(string, int64), void, "readonly">
/// @type.symbol symbol=symbol11.text source=text type=string
/// @type.symbol symbol=symbol11.count source=count type=int64

    const pair = (text, count);
    /// @type.symbol symbol=symbol11.pair source=pair type=(string, int64)
    /// @resolution.pattern source=pair kind=binding target=symbol11.pair
    /// @resolution.name source=text target=symbol11.text
    /// @resolution.place source=text placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=text root=symbol11.text
    /// @resolution.name source=count target=symbol11.count
    /// @resolution.place source=count placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=count root=symbol11.count

});

each<(int32,)>([(1,)])("one", (value: int32) => {});
/// @resolution.name source=each target=each
/// @resolution.call source="each<(int32,)>([(1,)])(\"one\", (value: int32) => {})" parameters=(string, (...(int32,)) => void) arguments=(provided("one") as string, provided((value: int32) => {}) as (...(int32,)) => void) return=void kind=expression target=expression
/// @resolution.call source=each<(int32,)>([(1,)]) parameters=((int32,)[]) arguments=(provided([(1,)]) as (int32,)[]) return=(string, (...(int32,)) => void) => void kind=symbol target=each instance=each<(int32,)>
/// @generic.instantiation id=each<(int32,)> template=each arguments=((int32,))
/// @resolution.call source=[(1,)] parameters=(^Slice<(int32,)>) arguments=(rest(provided((1,)) as (int32,)) as (int32,)) return=(int32,)[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<(int32,)>
/// @generic.instantiation id=arrayFromOwnedSlice<(int32,)> template=arrayFromOwnedSlice arguments=((int32,))
/// @type.symbol symbol=symbol15 source="(value: int32) => {}" type=Function<(int32,), void, "readonly">
/// @type.symbol symbol=symbol15.value source="value: int32" type=int32

each([(1,)])("closed", (value: int32) => {});
/// @resolution.name source=each target=each
/// @resolution.call source="each([(1,)])(\"closed\", (value: int32) => {})" parameters=(string, (...(int32,)) => void) arguments=(provided("closed") as string, provided((value: int32) => {}) as (...(int32,)) => void) return=void kind=expression target=expression
/// @resolution.call source=each([(1,)]) parameters=((int32,)[]) arguments=(provided([(1,)]) as (int32,)[]) return=(string, (...(int32,)) => void) => void kind=symbol target=each instance=each<(int32,)>
/// @resolution.call source=[(1,)] parameters=(^Slice<(int32,)>) arguments=(rest(provided((1,)) as (int32,)) as (int32,)) return=(int32,)[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<(int32,)>
/// @type.symbol symbol=symbol17 source="(value: int32) => {}" type=Function<(int32,), void, "readonly">
/// @type.symbol symbol=symbol17.value source="value: int32" type=int32
"#,
        r#"

"#,
    );
}

#[test]
fn test_type_a_callback_parameter_before_its_body_under_an_open_result() {
    let session = TestSession::single(
        r#"
import { Result } from "tspp:error";

function normalize(result: Result<int32, string>): Result<int32, string> {
    return result.andThen((value) => {
        if (value < 0) {
            return Result.ok(-value);
        }

        return Result.ok(value);
    });
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { Result } from "tspp:error";

function normalize(result: Result<int32, string>): Result<int32, string> {
    return result.andThen<int32, string, int32, string>((value: int32): Result<int32, string> => {
        if (value < 0) {
            return Result.ok<int32, string>(-value);
        }

        return Result.ok<int32, string>(value);
    });
}

=== dir ===
import { Result } from "tspp:error";

function normalize(result: Result<int32, string>): Result<int32, string> {
/// @type.symbol symbol=normalize type=(Result<int32, string>) => Result<int32, string>
/// @type.symbol symbol=normalize.result source="result: Result<int32, string>" type=Result<int32, string>
/// @resolution.name source=Result target=Result
/// @resolution.name source=Result target=Result

    return result.andThen((value) => {
    /// @resolution.name source=result target=normalize.result
    /// @resolution.member source=result.andThen receiver=Result<int32, string> type=<andThen.U, andThen.F>(this: Result<int32, string>, (int32) => Result<andThen.U, andThen.F>) => Result<andThen.U, string | andThen.F> kind=symbol target_receiver=Result<int32, string> target=andThen
    /// @resolution.call parameters=((int32) => Result<int32, string>) arguments=(provided(argument) as (int32) => Result<int32, string>) return=Result<int32, string> kind=symbol target=andThen receiver=Result<int32, string> instance="Result<int32, string>.<extension#1>.andThen<int32, string>"
    /// @resolution.place source=result placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=result root=normalize.result
    /// @generic.instantiation id="andThen<int32, string, int32, string>" template=andThen arguments=(int32, string, int32, string)
    /// @generic.instantiation id="andThen<int32, string>" template=andThen arguments=(int32, string)
    /// @type.symbol symbol=normalize.symbol4 type=Function<(int32,), Result<int32, string>, "readonly">
    /// @type.symbol symbol=normalize.symbol4.value source=value type=int32

        if (value < 0) {
        /// @resolution.name source=value target=normalize.symbol4.value
        /// @resolution.operator source="value < 0" type=boolean operator="<" kind=builtin operands=[value as int32 families=(integer), 0 as int32 families=(integer)]
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=normalize.symbol4.value

            return Result.ok(-value);
            /// @resolution.name source=Result target=Result
            /// @resolution.member source=Result.ok receiver=Result type=(T#1) => Result<T#1, E#1> kind=symbol target_receiver=Result target=ok#1
            /// @resolution.call source=Result.ok(-value) parameters=(int32) arguments=(provided(-value) as int32) return=Result<int32, string> kind=symbol target=ok#1 instance="Result<int32, string>.<extension#1>.ok#1"
            /// @generic.instantiation id="ok#1<int32, string>" template=ok#1 arguments=(int32, string)
            /// @resolution.operator source=-value type=int32 operator="-" kind=builtin operands=[value as int32 families=(integer)]
            /// @resolution.name source=value target=normalize.symbol4.value
            /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=value root=normalize.symbol4.value

        }

        return Result.ok(value);
        /// @resolution.name source=Result target=Result
        /// @resolution.member source=Result.ok receiver=Result type=(T#1) => Result<T#1, E#1> kind=symbol target_receiver=Result target=ok#1
        /// @resolution.call source=Result.ok(value) parameters=(int32) arguments=(provided(value) as int32) return=Result<int32, string> kind=symbol target=ok#1 instance="Result<int32, string>.<extension#1>.ok#1"
        /// @resolution.name source=value target=normalize.symbol4.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=normalize.symbol4.value

    });
}
"#,
        r#"

"#,
    );
}

#[test]
fn test_infer_parameterized_test_closures_from_their_case_tuples() {
    let session = TestSession::single(
        r#"
import { describe, test } from "tspp:test";

test.each([(1,)]).only("parameterized case", (value: int32) => {});
test.for([1]).only("table case", (value: &readonly int64) => {});
describe.each([(1,)])("parameterized suite", (value: int32) => {});
describe.for([1])("table suite", (value: int32) => {});
test.each([("a", 2)])("pairs", (text, count) => {
    const pair = (text, count);
});
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { describe, test } from "tspp:test";

test.each<{}, {}, {}, (int32,)>([(1,)] as Iterable<(int32,)>).only(
    "parameterized case",
    ((value: int32): BodyResult => {}) as Function<(int32,), BodyResult> | undefined,
);
test.for<{}, {}, {}, int64>([1] as Iterable<int64>).only(
    "table case",
    <'a,>(value: &'a readonly int64): BodyResult => {},
);
describe.each<(int32,)>([(1,)] as Iterable<(int32,)>)("parameterized suite", ((
    value: int32
): void => {}) as Function<(int32,), void> | undefined);
describe.for<int32>([1] as Iterable<int32>)("table suite", ((value: int32): void => {}) as | ((
      value: int32,
  ) => void)
| undefined);
test.each<{}, {}, {}, (string, int64)>([("a", 2)] as Iterable<(string, int64)>)("pairs", ((
    text: string,
    count: int64,
): BodyResult => {
    const pair: (string, int64) = (text, count);
}) as Function<(string, int64), BodyResult> | undefined);

=== dir ===
import { describe, test } from "tspp:test";

test.each([(1,)]).only("parameterized case", (value: int32) => {});
/// @resolution.name source=test target=test
/// @resolution.member source=test.each receiver=Test<{}, {}, {}> type=<Test.each.P: (...unknown[],) & Copy>(Iterable<Test.each.P>) => ParameterizedTest<Test.each.P> kind=symbol target_receiver=Test<{}, {}, {}> dispatch=dynamic constraint=Test<{}, {}, {}> target=Test.each
/// @resolution.member source=test.each([(1,)]).only receiver=ParameterizedTest<(int32,)> type=ParameterizedTest<(int32,)> kind=field target_receiver=ParameterizedTest<(int32,)> dispatch=dynamic constraint=ParameterizedTest<(int32,)> key=only target=ParameterizedTest.only target_type=ParameterizedTest<(int32,)>
/// @resolution.call source="test.each([(1,)]).only(\"parameterized case\", (value: int32) => {})" parameters=(string, (int32) => BodyResult | undefined) arguments=(provided("parameterized case") as string, provided((value: int32) => {}) as (int32) => BodyResult | undefined) return=void kind=dynamic target=call(type_member) receiver=ParameterizedTest<(int32,)> constraint=ParameterizedTest<(int32,)>
/// @resolution.call source=test.each([(1,)]) parameters=(Iterable<(int32,)>) arguments=(provided([(1,)]) as Iterable<(int32,)>) return=ParameterizedTest<(int32,)> kind=dynamic target=Test.each receiver=Test<{}, {}, {}> constraint=Test<{}, {}, {}> generic_arguments=({}, {}, {}, (int32,))
/// @resolution.place source=test placement="local" lifetime="static" access="immutable"
/// @resolution.access source=test root=test
/// @resolution.place source=test.each([(1,)]).only placement="local" lifetime="managed" access="mutable"
/// @generic.instantiation id="Test.each<{}, {}, {}>" template=Test.each arguments=({}, {}, {})
/// @resolution.call source=[(1,)] parameters=(^Slice<(int32,)>) arguments=(rest(provided((1,)) as (int32,)) as (int32,)) return=(int32,)[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<(int32,)>
/// @generic.instantiation id=arrayFromOwnedSlice<(int32,)> template=arrayFromOwnedSlice arguments=((int32,))
/// @type.symbol symbol=symbol3 source="(value: int32) => {}" type=Function<(int32,), BodyResult, "readonly">
/// @type.symbol symbol=symbol3.value source="value: int32" type=int32

test.for([1]).only("table case", (value: &readonly int64) => {});
/// @resolution.name source=test target=test
/// @resolution.member source=test.for receiver=Test<{}, {}, {}> type=<Test.for.T>(Iterable<Test.for.T>) => TableTest<Test.for.T, {}, {}, {}> kind=symbol target_receiver=Test<{}, {}, {}> dispatch=dynamic constraint=Test<{}, {}, {}> target=Test.for
/// @resolution.member source=test.for([1]).only receiver=TableTest<int64, {}, {}, {}> type=TableTest<int64, {}, {}, {}> kind=field target_receiver=TableTest<int64, {}, {}, {}> dispatch=dynamic constraint=TableTest<int64, {}, {}, {}> key=only target=TableTest.only target_type=TableTest<int64, {}, {}, {}>
/// @resolution.call source="test.for([1]).only(\"table case\", (value: &readonly int64) => {})" parameters=(string, <type_expression.'a, type_expression.'b>(&type_expression.'a readonly int64, &type_expression.'b TestContext<{}, {}, {}>) => BodyResult | undefined) arguments=(provided("table case") as string, provided((value: &readonly int64) => {}) as <type_expression.'a, type_expression.'b>(&type_expression.'a readonly int64, &type_expression.'b TestContext<{}, {}, {}>) => BodyResult | undefined) return=void kind=dynamic target=call(type_member) receiver=TableTest<int64, {}, {}, {}> constraint=TableTest<int64, {}, {}, {}>
/// @resolution.call source=test.for([1]) parameters=(Iterable<int64>) arguments=(provided([1]) as Iterable<int64>) return=TableTest<int64, {}, {}, {}> kind=dynamic target=Test.for receiver=Test<{}, {}, {}> constraint=Test<{}, {}, {}> generic_arguments=({}, {}, {}, int64)
/// @resolution.place source=test placement="local" lifetime="static" access="immutable"
/// @resolution.access source=test root=test
/// @resolution.place source=test.for([1]).only placement="local" lifetime="managed" access="mutable"
/// @generic.instantiation id="Test.for<{}, {}, {}>" template=Test.for arguments=({}, {}, {})
/// @resolution.call source=[1] parameters=(^Slice<int64>) arguments=(rest(provided(1) as int64) as int64) return=int64[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int64>
/// @generic.instantiation id=arrayFromOwnedSlice<int64> template=arrayFromOwnedSlice arguments=(int64)
/// @generic.template symbol=symbol5 parameters=('a)
/// @type.symbol symbol=symbol5 source="(value: &readonly int64) => {}" type=Function<(&symbol5.'a readonly int64,), BodyResult, "readonly">
/// @type.symbol symbol=symbol5.value source="value: &readonly int64" type=&symbol5.'a readonly int64

describe.each([(1,)])("parameterized suite", (value: int32) => {});
/// @resolution.name source=describe target=describe
/// @resolution.member source=describe.each receiver=TestSuite type=<TestSuite.each.P: (...unknown[],)>(Iterable<TestSuite.each.P>) => ParameterizedSuite<TestSuite.each.P> kind=symbol target_receiver=TestSuite dispatch=dynamic constraint=TestSuite target=TestSuite.each
/// @resolution.call source="describe.each([(1,)])(\"parameterized suite\", (value: int32) => {})" parameters=(string, (int32) => void | undefined) arguments=(provided("parameterized suite") as string, provided((value: int32) => {}) as (int32) => void | undefined) return=void kind=dynamic target=call(type_member) receiver=ParameterizedSuite<(int32,)> constraint=ParameterizedSuite<(int32,)>
/// @resolution.call source=describe.each([(1,)]) parameters=(Iterable<(int32,)>) arguments=(provided([(1,)]) as Iterable<(int32,)>) return=ParameterizedSuite<(int32,)> kind=dynamic target=TestSuite.each receiver=TestSuite constraint=TestSuite generic_arguments=((int32,))
/// @resolution.place source=describe placement="local" lifetime="static" access="immutable"
/// @resolution.access source=describe root=describe
/// @resolution.call source=[(1,)] parameters=(^Slice<(int32,)>) arguments=(rest(provided((1,)) as (int32,)) as (int32,)) return=(int32,)[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<(int32,)>
/// @type.symbol symbol=symbol7 source="(value: int32) => {}" type=Function<(int32,), void, "readonly">
/// @type.symbol symbol=symbol7.value source="value: int32" type=int32

describe.for([1])("table suite", (value: int32) => {});
/// @resolution.name source=describe target=describe
/// @resolution.member source=describe.for receiver=TestSuite type=<TestSuite.for.T>(Iterable<TestSuite.for.T>) => TableSuite<TestSuite.for.T> kind=symbol target_receiver=TestSuite dispatch=dynamic constraint=TestSuite target=TestSuite.for
/// @resolution.call source="describe.for([1])(\"table suite\", (value: int32) => {})" parameters=(string, (int32) => void | undefined) arguments=(provided("table suite") as string, provided((value: int32) => {}) as (int32) => void | undefined) return=void kind=dynamic target=call(type_member) receiver=TableSuite<int32> constraint=TableSuite<int32>
/// @resolution.call source=describe.for([1]) parameters=(Iterable<int32>) arguments=(provided([1]) as Iterable<int32>) return=TableSuite<int32> kind=dynamic target=TestSuite.for receiver=TestSuite constraint=TestSuite generic_arguments=(int32)
/// @resolution.place source=describe placement="local" lifetime="static" access="immutable"
/// @resolution.access source=describe root=describe
/// @resolution.call source=[1] parameters=(^Slice<int32>) arguments=(rest(provided(1) as int32) as int32) return=int32[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int32>
/// @generic.instantiation id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
/// @type.symbol symbol=symbol9 source="(value: int32) => {}" type=Function<(int32,), void, "readonly">
/// @type.symbol symbol=symbol9.value source="value: int32" type=int32

test.each([("a", 2)])("pairs", (text, count) => {
/// @resolution.name source=test target=test
/// @resolution.member source=test.each receiver=Test<{}, {}, {}> type=<Test.each.P: (...unknown[],) & Copy>(Iterable<Test.each.P>) => ParameterizedTest<Test.each.P> kind=symbol target_receiver=Test<{}, {}, {}> dispatch=dynamic constraint=Test<{}, {}, {}> target=Test.each
/// @resolution.call parameters=(string, (string, int64) => BodyResult | undefined) arguments=(provided("pairs") as string, provided(argument) as (string, int64) => BodyResult | undefined) return=void kind=dynamic target=call(type_member) receiver=ParameterizedTest<(string, int64)> constraint=ParameterizedTest<(string, int64)>
/// @resolution.call source="test.each([(\"a\", 2)])" parameters=(Iterable<(string, int64)>) arguments=(provided([("a", 2)]) as Iterable<(string, int64)>) return=ParameterizedTest<(string, int64)> kind=dynamic target=Test.each receiver=Test<{}, {}, {}> constraint=Test<{}, {}, {}> generic_arguments=({}, {}, {}, (string, int64))
/// @resolution.place source=test placement="local" lifetime="static" access="immutable"
/// @resolution.access source=test root=test
/// @resolution.call source=[("a", 2)] parameters=(^Slice<(string, int64)>) arguments=(rest(provided(("a", 2)) as (string, int64)) as (string, int64)) return=(string, int64)[] kind=symbol target=arrayFromOwnedSlice instance="arrayFromOwnedSlice<(string, int64)>"
/// @generic.instantiation id="arrayFromOwnedSlice<(string, int64)>" template=arrayFromOwnedSlice arguments=((string, int64))
/// @type.symbol symbol=symbol11 type=Function<(string, int64), BodyResult, "readonly">
/// @type.symbol symbol=symbol11.text source=text type=string
/// @type.symbol symbol=symbol11.count source=count type=int64

    const pair = (text, count);
    /// @type.symbol symbol=symbol11.pair source=pair type=(string, int64)
    /// @resolution.pattern source=pair kind=binding target=symbol11.pair
    /// @resolution.name source=text target=symbol11.text
    /// @resolution.place source=text placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=text root=symbol11.text
    /// @resolution.name source=count target=symbol11.count
    /// @resolution.place source=count placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=count root=symbol11.count

});
"#,
        r#"

"#,
    );
}

#[test]
fn test_flow_the_declared_result_into_a_conditional_callback_body() {
    let session = TestSession::single(
        r#"
function parse(result: Result<string, string>): Result<int32, string> {
    return result.andThen((value) =>
        value.isEmpty ? Result.err("empty") : Result.ok(1)
    );
}

function widen(result: Result<string, string>): Result<int32, string> {
    return result.andThen((value) => {
        if (value.isEmpty) {
            return Result.err("empty");
        }

        return Result.ok(1);
    });
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
function parse(result: Result<string, string>): Result<int32, string> {
    return result.andThen<string, string, int32, string>(
        (value: string): Result<int32, string> =>
            value.isEmpty ? Result.err<int32, string>("empty") : Result.ok<int32, string>(1),
    );
}

function widen(result: Result<string, string>): Result<int32, string> {
    return result.andThen<string, string, int32, string>((value: string): Result<int32, string> => {
        if (value.isEmpty) {
            return Result.err<int32, string>("empty");
        }

        return Result.ok<int32, string>(1);
    });
}

=== dir ===
function parse(result: Result<string, string>): Result<int32, string> {
/// @type.symbol symbol=parse type=(Result<string, string>) => Result<int32, string>
/// @type.symbol symbol=parse.result source="result: Result<string, string>" type=Result<string, string>
/// @resolution.name source=Result target=Result
/// @resolution.name source=Result target=Result

    return result.andThen((value) =>
    /// @resolution.name source=result target=parse.result
    /// @resolution.member source=result.andThen receiver=Result<string, string> type=<andThen.U, andThen.F>(this: Result<string, string>, (string) => Result<andThen.U, andThen.F>) => Result<andThen.U, string | andThen.F> kind=symbol target_receiver=Result<string, string> target=andThen
    /// @resolution.call parameters=((string) => Result<int32, string>) arguments=(provided(argument) as (string) => Result<int32, string>) return=Result<int32, string> kind=symbol target=andThen receiver=Result<string, string> instance="Result<string, string>.<extension#1>.andThen<int32, string>"
    /// @resolution.place source=result placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=result root=parse.result
    /// @generic.instantiation id="andThen<string, string, int32, string>" template=andThen arguments=(string, string, int32, string)
    /// @generic.instantiation id="andThen<string, string>" template=andThen arguments=(string, string)
    /// @type.symbol symbol=parse.symbol3 type=Function<(string,), Result<int32, string>, "readonly">
    /// @type.symbol symbol=parse.symbol3.value source=value type=string

        value.isEmpty ? Result.err("empty") : Result.ok(1)
        /// @resolution.name source=value target=parse.symbol3.value
        /// @resolution.member source=value.isEmpty receiver=string type=boolean kind=call target="isEmpty(parameters=(), arguments=(), return=boolean, regions=(\"managed\" & \"local\"))"
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=parse.symbol3.value
        /// @generic.instantiation id="isEmpty<\"managed\" & \"local\">" template=isEmpty arguments=("managed" & "local")
        /// @resolution.name source=Result target=Result
        /// @resolution.member source=Result.err receiver=Result type=(E#1) => Result<T#1, E#1> kind=symbol target_receiver=Result target=err#1
        /// @resolution.call source="Result.err(\"empty\")" parameters=(string) arguments=(provided("empty") as string) return=Result<int32, string> kind=symbol target=err#1 instance="Result<int32, string>.<extension#1>.err#1"
        /// @generic.instantiation id="err#1<int32, string>" template=err#1 arguments=(int32, string)
        /// @resolution.name source=Result target=Result
        /// @resolution.member source=Result.ok receiver=Result type=(T#1) => Result<T#1, E#1> kind=symbol target_receiver=Result target=ok#1
        /// @resolution.call source=Result.ok(1) parameters=(int32) arguments=(provided(1) as int32) return=Result<int32, string> kind=symbol target=ok#1 instance="Result<int32, string>.<extension#1>.ok#1"
        /// @generic.instantiation id="ok#1<int32, string>" template=ok#1 arguments=(int32, string)

    );
}

function widen(result: Result<string, string>): Result<int32, string> {
/// @type.symbol symbol=widen type=(Result<string, string>) => Result<int32, string>
/// @type.symbol symbol=widen.result source="result: Result<string, string>" type=Result<string, string>
/// @resolution.name source=Result target=Result
/// @resolution.name source=Result target=Result

    return result.andThen((value) => {
    /// @resolution.name source=result target=widen.result
    /// @resolution.member source=result.andThen receiver=Result<string, string> type=<andThen.U, andThen.F>(this: Result<string, string>, (string) => Result<andThen.U, andThen.F>) => Result<andThen.U, string | andThen.F> kind=symbol target_receiver=Result<string, string> target=andThen
    /// @resolution.call parameters=((string) => Result<int32, string>) arguments=(provided(argument) as (string) => Result<int32, string>) return=Result<int32, string> kind=symbol target=andThen receiver=Result<string, string> instance="Result<string, string>.<extension#1>.andThen<int32, string>"
    /// @resolution.place source=result placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=result root=widen.result
    /// @type.symbol symbol=widen.symbol7 type=Function<(string,), Result<int32, string>, "readonly">
    /// @type.symbol symbol=widen.symbol7.value source=value type=string

        if (value.isEmpty) {
        /// @resolution.name source=value target=widen.symbol7.value
        /// @resolution.member source=value.isEmpty receiver=string type=boolean kind=call target="isEmpty(parameters=(), arguments=(), return=boolean, regions=(\"managed\" & \"local\"))"
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=widen.symbol7.value

            return Result.err("empty");
            /// @resolution.name source=Result target=Result
            /// @resolution.member source=Result.err receiver=Result type=(E#1) => Result<T#1, E#1> kind=symbol target_receiver=Result target=err#1
            /// @resolution.call source="Result.err(\"empty\")" parameters=(string) arguments=(provided("empty") as string) return=Result<int32, string> kind=symbol target=err#1 instance="Result<int32, string>.<extension#1>.err#1"

        }

        return Result.ok(1);
        /// @resolution.name source=Result target=Result
        /// @resolution.member source=Result.ok receiver=Result type=(T#1) => Result<T#1, E#1> kind=symbol target_receiver=Result target=ok#1
        /// @resolution.call source=Result.ok(1) parameters=(int32) arguments=(provided(1) as int32) return=Result<int32, string> kind=symbol target=ok#1 instance="Result<int32, string>.<extension#1>.ok#1"

    });
}
"#,
        r#"

"#,
    );
}

/// Bind a pack instantiation from the closure annotated after it.
#[test]
fn test_bind_a_pack_instantiation_from_a_later_annotated_closure() {
    let session = TestSession::single(
        r#"
declare function run<P: (...unknown[],)>(cases: P[]): (name: string, body?: Function<P, void>) => void;
declare function plain<P: (...unknown[],)>(cases: P[]): (name: string, body: Function<P, void>) => void;
declare function table<T>(values: T[]): <'a>(name: string, body: (value: &'a readonly T) => void) => void;
declare function optionalTable<T>(values: T[]): <'a, 'b>(name: string, body?: (value: &'a readonly T, context: &'b string) => void) => void;
declare function iterTable<T>(values: Iterable<T>): <'a>(name: string, body: (value: &'a readonly T) => void) => void;
declare function contextTable<T>(values: T[]): <'a, 'b>(name: string, body: (value: &'a readonly T, context: &'b string) => void) => void;

run([(1,)])("optional", (value: int32) => {});
plain([(1,)])("plain", (value: int32) => {});
table([1])("table", (value: &readonly int32) => {});
optionalTable([1])("optional table", (value: &readonly int32) => {});
contextTable([1])("context table", (value: &readonly int32) => {});
iterTable([1])("iter table", (value: &readonly int32) => {});
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
declare function run<P: (...unknown[],)>(
    cases: P[],
): (name: string, body?: Function<P, void>) => void;
declare function plain<P: (...unknown[],)>(
    cases: P[],
): (name: string, body: Function<P, void>) => void;
declare function table<T>(
    values: T[],
): <'a>(name: string, body: (value: &'a readonly T) => void) => void;
declare function optionalTable<T>(
    values: T[],
): <'a, 'b>(name: string, body?: (value: &'a readonly T, context: &'b string) => void) => void;
declare function iterTable<T>(
    values: Iterable<T>,
): <'a>(name: string, body: (value: &'a readonly T) => void) => void;
declare function contextTable<T>(
    values: T[],
): <'a, 'b>(name: string, body: (value: &'a readonly T, context: &'b string) => void) => void;

run<(int32,)>([(1,)])("optional", ((value: int32): void => {}) as | Function<(int32,), void>
| undefined);
plain<(int32,)>([(1,)])("plain", (value: int32): void => {});
table<int32>([1])<'a>("table", <'a,>(value: &'a readonly int32): void => {});
optionalTable<int32>([1])<'a, "frame">("optional table", (<'a,>(
    value: &'a readonly int32
): void => {}) as ((value: &'a readonly int32, context: &'frame string) => void) | undefined);
contextTable<int32>([1])<'a, "frame">(
    "context table",
    <'a,>(value: &'a readonly int32): void => {},
);
iterTable<int32>([1] as Iterable<int32>)<'a>(
    "iter table",
    <'a,>(value: &'a readonly int32): void => {},
);

=== dir ===
declare function run<P: (...unknown[],)>(cases: P[]): (name: string, body?: Function<P, void>) => void;
/// @generic.template symbol=run parameters=(P#1: (...unknown[],))
/// @type.symbol symbol=run type=<P#1: (...unknown[],)>(P#1[]) => (string, Function<P#1, void, "mutable"> | undefined?) => void
/// @type.symbol symbol=run.P source="P: (...unknown[],)" type=P#1
/// @resolution.name source=P target=run.P
/// @type.symbol symbol=run.name source="name: string" type=string
/// @type.symbol symbol=run.body source="body?: Function<P, void>" type=Function<P#1, void, "mutable"> | undefined
/// @resolution.name source=Function target=Function
/// @resolution.name source=P target=run.P

declare function plain<P: (...unknown[],)>(cases: P[]): (name: string, body: Function<P, void>) => void;
/// @generic.template symbol=plain parameters=(P#2: (...unknown[],))
/// @type.symbol symbol=plain type=<P#2: (...unknown[],)>(P#2[]) => (string, Function<P#2, void, "mutable">) => void
/// @type.symbol symbol=plain.P source="P: (...unknown[],)" type=P#2
/// @resolution.name source=P target=plain.P
/// @type.symbol symbol=plain.name source="name: string" type=string
/// @type.symbol symbol=plain.body source="body: Function<P, void>" type=Function<P#2, void, "mutable">
/// @resolution.name source=Function target=Function
/// @resolution.name source=P target=plain.P

declare function table<T>(values: T[]): <'a>(name: string, body: (value: &'a readonly T) => void) => void;
/// @generic.template symbol=table parameters=(T#1)
/// @type.symbol symbol=table type=<T#1>(T#1[]) => <'a#1>(string, (&'a#1 readonly T#1) => void) => void
/// @type.symbol symbol=table.T source=T type=T#1
/// @resolution.name source=T target=table.T
/// @generic.template source=type_expression parent=template#2 parameters=('a#1)
/// @type.symbol symbol=table.'a source='a type='a#1
/// @type.symbol symbol=table.name source="name: string" type=string
/// @type.symbol symbol=table.body source="body: (value: &'a readonly T) => void" type=(&'a#1 readonly T#1) => void
/// @type.symbol symbol=table.value source="value: &'a readonly T" type=&'a#1 readonly T#1
/// @resolution.name source='a target=table.'a
/// @resolution.name source=T target=table.T

declare function optionalTable<T>(values: T[]): <'a, 'b>(name: string, body?: (value: &'a readonly T, context: &'b string) => void) => void;
/// @generic.template symbol=optionalTable parameters=(T#2)
/// @type.symbol symbol=optionalTable type=<T#2>(T#2[]) => <'a#2, 'b#1>(string, (&'a#2 readonly T#2, &'b#1 string) => void | undefined?) => void
/// @type.symbol symbol=optionalTable.T source=T type=T#2
/// @resolution.name source=T target=optionalTable.T
/// @generic.template source=type_expression parent=template#3 parameters=('a#2, 'b#1)
/// @type.symbol symbol=optionalTable.'a source='a type='a#2
/// @type.symbol symbol=optionalTable.'b source='b type='b#1
/// @type.symbol symbol=optionalTable.name source="name: string" type=string
/// @type.symbol symbol=optionalTable.body source="body?: (value: &'a readonly T, context: &'b string) => void" type=(&'a#2 readonly T#2, &'b#1 string) => void | undefined
/// @type.symbol symbol=optionalTable.value source="value: &'a readonly T" type=&'a#2 readonly T#2
/// @resolution.name source='a target=optionalTable.'a
/// @resolution.name source=T target=optionalTable.T
/// @type.symbol symbol=optionalTable.context source="context: &'b string" type=&'b#1 string
/// @resolution.name source='b target=optionalTable.'b

declare function iterTable<T>(values: Iterable<T>): <'a>(name: string, body: (value: &'a readonly T) => void) => void;
/// @generic.template symbol=iterTable parameters=(T#3)
/// @type.symbol symbol=iterTable type=<T#3>(Iterable<T#3>) => <'a#3>(string, (&'a#3 readonly T#3) => void) => void
/// @type.symbol symbol=iterTable.T source=T type=T#3
/// @resolution.name source=Iterable target=Iterable
/// @resolution.name source=T target=iterTable.T
/// @generic.template source=type_expression parent=template#4 parameters=('a#3)
/// @type.symbol symbol=iterTable.'a source='a type='a#3
/// @type.symbol symbol=iterTable.name source="name: string" type=string
/// @type.symbol symbol=iterTable.body source="body: (value: &'a readonly T) => void" type=(&'a#3 readonly T#3) => void
/// @type.symbol symbol=iterTable.value source="value: &'a readonly T" type=&'a#3 readonly T#3
/// @resolution.name source='a target=iterTable.'a
/// @resolution.name source=T target=iterTable.T

declare function contextTable<T>(values: T[]): <'a, 'b>(name: string, body: (value: &'a readonly T, context: &'b string) => void) => void;
/// @generic.template symbol=contextTable parameters=(T#4)
/// @type.symbol symbol=contextTable type=<T#4>(T#4[]) => <'a#4, 'b#2>(string, (&'a#4 readonly T#4, &'b#2 string) => void) => void
/// @type.symbol symbol=contextTable.T source=T type=T#4
/// @resolution.name source=T target=contextTable.T
/// @generic.template source=type_expression parent=template#5 parameters=('a#4, 'b#2)
/// @type.symbol symbol=contextTable.'a source='a type='a#4
/// @type.symbol symbol=contextTable.'b source='b type='b#2
/// @type.symbol symbol=contextTable.name source="name: string" type=string
/// @type.symbol symbol=contextTable.body source="body: (value: &'a readonly T, context: &'b string) => void" type=(&'a#4 readonly T#4, &'b#2 string) => void
/// @type.symbol symbol=contextTable.value source="value: &'a readonly T" type=&'a#4 readonly T#4
/// @resolution.name source='a target=contextTable.'a
/// @resolution.name source=T target=contextTable.T
/// @type.symbol symbol=contextTable.context source="context: &'b string" type=&'b#2 string
/// @resolution.name source='b target=contextTable.'b

run([(1,)])("optional", (value: int32) => {});
/// @resolution.name source=run target=run
/// @resolution.call source="run([(1,)])(\"optional\", (value: int32) => {})" parameters=(string, (int32) => void | undefined) arguments=(provided("optional") as string, provided((value: int32) => {}) as (int32) => void | undefined) return=void kind=expression target=expression
/// @resolution.call source=run([(1,)]) parameters=((int32,)[]) arguments=(provided([(1,)]) as (int32,)[]) return=(string, (int32) => void | undefined?) => void kind=symbol target=run instance=run<(int32,)>
/// @generic.instantiation id=run<(int32,)> template=run arguments=((int32,))
/// @resolution.call source=[(1,)] parameters=(^Slice<(int32,)>) arguments=(rest(provided((1,)) as (int32,)) as (int32,)) return=(int32,)[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<(int32,)>
/// @generic.instantiation id=arrayFromOwnedSlice<(int32,)> template=arrayFromOwnedSlice arguments=((int32,))
/// @type.symbol symbol=symbol43 source="(value: int32) => {}" type=Function<(int32,), void, "readonly">
/// @type.symbol symbol=symbol43.value source="value: int32" type=int32

plain([(1,)])("plain", (value: int32) => {});
/// @resolution.name source=plain target=plain
/// @resolution.call source="plain([(1,)])(\"plain\", (value: int32) => {})" parameters=(string, (int32) => void) arguments=(provided("plain") as string, provided((value: int32) => {}) as (int32) => void) return=void kind=expression target=expression
/// @resolution.call source=plain([(1,)]) parameters=((int32,)[]) arguments=(provided([(1,)]) as (int32,)[]) return=(string, (int32) => void) => void kind=symbol target=plain instance=plain<(int32,)>
/// @generic.instantiation id=plain<(int32,)> template=plain arguments=((int32,))
/// @resolution.call source=[(1,)] parameters=(^Slice<(int32,)>) arguments=(rest(provided((1,)) as (int32,)) as (int32,)) return=(int32,)[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<(int32,)>
/// @type.symbol symbol=symbol45 source="(value: int32) => {}" type=Function<(int32,), void, "readonly">
/// @type.symbol symbol=symbol45.value source="value: int32" type=int32

table([1])("table", (value: &readonly int32) => {});
/// @resolution.name source=table target=table
/// @resolution.call source="table([1])(\"table\", (value: &readonly int32) => {})" parameters=(string, (&symbol47.'a readonly int32) => void) arguments=(provided("table") as string, provided((value: &readonly int32) => {}) as (&symbol47.'a readonly int32) => void) return=void regions=(symbol47.'a) kind=expression target=expression generic_arguments=(symbol47.'a)
/// @resolution.call source=table([1]) parameters=(int32[]) arguments=(provided([1]) as int32[]) return=<'a#1>(string, (&'a#1 readonly int32) => void) => void kind=symbol target=table instance=table<int32>
/// @generic.instantiation id=table<int32> template=table arguments=(int32)
/// @resolution.call source=[1] parameters=(^Slice<int32>) arguments=(rest(provided(1) as int32) as int32) return=int32[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int32>
/// @generic.instantiation id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
/// @generic.template symbol=symbol47 parameters=('a)
/// @type.symbol symbol=symbol47 source="(value: &readonly int32) => {}" type=Function<(&symbol47.'a readonly int32,), void, "readonly">
/// @type.symbol symbol=symbol47.value source="value: &readonly int32" type=&symbol47.'a readonly int32

optionalTable([1])("optional table", (value: &readonly int32) => {});
/// @resolution.name source=optionalTable target=optionalTable
/// @resolution.call source="optionalTable([1])(\"optional table\", (value: &readonly int32) => {})" parameters=(string, (&symbol49.'a readonly int32, &'frame string) => void | undefined) arguments=(provided("optional table") as string, provided((value: &readonly int32) => {}) as (&symbol49.'a readonly int32, &'frame string) => void | undefined) return=void regions=(symbol49.'a, "frame") kind=expression target=expression generic_arguments=(symbol49.'a, "frame")
/// @resolution.call source=optionalTable([1]) parameters=(int32[]) arguments=(provided([1]) as int32[]) return=<'a#2, 'b#1>(string, (&'a#2 readonly int32, &'b#1 string) => void | undefined?) => void kind=symbol target=optionalTable instance=optionalTable<int32>
/// @generic.instantiation id=optionalTable<int32> template=optionalTable arguments=(int32)
/// @resolution.call source=[1] parameters=(^Slice<int32>) arguments=(rest(provided(1) as int32) as int32) return=int32[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int32>
/// @generic.template symbol=symbol49 parameters=('a)
/// @type.symbol symbol=symbol49 source="(value: &readonly int32) => {}" type=Function<(&symbol49.'a readonly int32,), void, "readonly">
/// @type.symbol symbol=symbol49.value source="value: &readonly int32" type=&symbol49.'a readonly int32

contextTable([1])("context table", (value: &readonly int32) => {});
/// @resolution.name source=contextTable target=contextTable
/// @resolution.call source="contextTable([1])(\"context table\", (value: &readonly int32) => {})" parameters=(string, (&symbol51.'a readonly int32, &'frame string) => void) arguments=(provided("context table") as string, provided((value: &readonly int32) => {}) as (&symbol51.'a readonly int32, &'frame string) => void) return=void regions=(symbol51.'a, "frame") kind=expression target=expression generic_arguments=(symbol51.'a, "frame")
/// @resolution.call source=contextTable([1]) parameters=(int32[]) arguments=(provided([1]) as int32[]) return=<'a#4, 'b#2>(string, (&'a#4 readonly int32, &'b#2 string) => void) => void kind=symbol target=contextTable instance=contextTable<int32>
/// @generic.instantiation id=contextTable<int32> template=contextTable arguments=(int32)
/// @resolution.call source=[1] parameters=(^Slice<int32>) arguments=(rest(provided(1) as int32) as int32) return=int32[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int32>
/// @generic.template symbol=symbol51 parameters=('a)
/// @type.symbol symbol=symbol51 source="(value: &readonly int32) => {}" type=Function<(&symbol51.'a readonly int32,), void, "readonly">
/// @type.symbol symbol=symbol51.value source="value: &readonly int32" type=&symbol51.'a readonly int32

iterTable([1])("iter table", (value: &readonly int32) => {});
/// @resolution.name source=iterTable target=iterTable
/// @resolution.call source="iterTable([1])(\"iter table\", (value: &readonly int32) => {})" parameters=(string, (&symbol53.'a readonly int32) => void) arguments=(provided("iter table") as string, provided((value: &readonly int32) => {}) as (&symbol53.'a readonly int32) => void) return=void regions=(symbol53.'a) kind=expression target=expression generic_arguments=(symbol53.'a)
/// @resolution.call source=iterTable([1]) parameters=(Iterable<int32>) arguments=(provided([1]) as Iterable<int32>) return=<'a#3>(string, (&'a#3 readonly int32) => void) => void kind=symbol target=iterTable instance=iterTable<int32>
/// @generic.instantiation id=iterTable<int32> template=iterTable arguments=(int32)
/// @resolution.call source=[1] parameters=(^Slice<int32>) arguments=(rest(provided(1) as int32) as int32) return=int32[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int32>
/// @generic.template symbol=symbol53 parameters=('a)
/// @type.symbol symbol=symbol53 source="(value: &readonly int32) => {}" type=Function<(&symbol53.'a readonly int32,), void, "readonly">
/// @type.symbol symbol=symbol53.value source="value: &readonly int32" type=&symbol53.'a readonly int32
"#,
        r#"

"#,
    );
}

#[test]
fn test_spread_packs_over_rest_optional_and_partially_annotated_closures() {
    let session = TestSession::single(
        r#"
declare function each<P: (...unknown[],) & Copy>(cases: P[]): (name: string, run: (...args: P) => void) => void;

each([(1, "a", true)])("triple", (count, text, flag) => {
    const triple = (count, text, flag);
});

each([(1, "a")])("rest", (...args) => {
    const pack = args;
});

each([(1, "a")])("partial", (count, text: string) => {
    const pair = (count, text);
});

each([(1,)])("typed", (count: int32) => {});
each([(2,)])("typed again", (count: int32) => {});
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
declare function each<P: (...unknown[],) & Copy>(
    cases: P[],
): (name: string, run: (...args: P) => void) => void;

each<(int64, string, boolean)>([(1, "a", true)])(
    "triple",
    (count: int64, text: string, flag: boolean): void => {
        const triple: (int64, string, boolean) = (count, text, flag);
    },
);

each<(int64, string)>([(1, "a")])("rest", (...args: (int64, string)): void => {
    const pack: (int64, string) = args;
});

each<(int64, string)>([(1, "a")])("partial", (count: int64, text: string): void => {
    const pair: (int64, string) = (count, text);
});

each<(int32,)>([(1,)])("typed", (count: int32): void => {});
each<(int32,)>([(2,)])("typed again", (count: int32): void => {});

=== dir ===
declare function each<P: (...unknown[],) & Copy>(cases: P[]): (name: string, run: (...args: P) => void) => void;
/// @generic.template symbol=each parameters=(P: (...unknown[],) & Copy)
/// @type.symbol symbol=each type=<P: (...unknown[],) & Copy>(P[]) => (string, (...P) => void) => void
/// @type.symbol symbol=each.P source="P: (...unknown[],) & Copy" type=P
/// @resolution.name source=Copy target=Copy
/// @resolution.name source=P target=each.P
/// @type.symbol symbol=each.name source="name: string" type=string
/// @type.symbol symbol=each.run source="run: (...args: P) => void" type=(...P) => void
/// @type.symbol symbol=each.args source="...args: P" type=P
/// @resolution.name source=P target=each.P

each([(1, "a", true)])("triple", (count, text, flag) => {
/// @resolution.name source=each target=each
/// @resolution.call parameters=(string, (...(int64, string, boolean)) => void) arguments=(provided("triple") as string, provided(argument) as (...(int64, string, boolean)) => void) return=void kind=expression target=expression
/// @resolution.call source="each([(1, \"a\", true)])" parameters=((int64, string, boolean)[]) arguments=(provided([(1, "a", true)]) as (int64, string, boolean)[]) return=(string, (...(int64, string, boolean)) => void) => void kind=symbol target=each instance="each<(int64, string, boolean)>"
/// @generic.instantiation id="each<(int64, string, boolean)>" template=each arguments=((int64, string, boolean))
/// @resolution.call source=[(1, "a", true)] parameters=(^Slice<(int64, string, boolean)>) arguments=(rest(provided((1, "a", true)) as (int64, string, boolean)) as (int64, string, boolean)) return=(int64, string, boolean)[] kind=symbol target=arrayFromOwnedSlice instance="arrayFromOwnedSlice<(int64, string, boolean)>"
/// @generic.instantiation id="arrayFromOwnedSlice<(int64, string, boolean)>" template=arrayFromOwnedSlice arguments=((int64, string, boolean))
/// @type.symbol symbol=symbol7 type=Function<(int64, string, boolean), void, "readonly">
/// @type.symbol symbol=symbol7.count source=count type=int64
/// @type.symbol symbol=symbol7.text source=text type=string
/// @type.symbol symbol=symbol7.flag source=flag type=boolean

    const triple = (count, text, flag);
    /// @type.symbol symbol=symbol7.triple source=triple type=(int64, string, boolean)
    /// @resolution.pattern source=triple kind=binding target=symbol7.triple
    /// @resolution.name source=count target=symbol7.count
    /// @resolution.place source=count placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=count root=symbol7.count
    /// @resolution.name source=text target=symbol7.text
    /// @resolution.place source=text placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=text root=symbol7.text
    /// @resolution.name source=flag target=symbol7.flag
    /// @resolution.place source=flag placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=flag root=symbol7.flag

});

each([(1, "a")])("rest", (...args) => {
/// @resolution.name source=each target=each
/// @resolution.call parameters=(string, (...(int64, string)) => void) arguments=(provided("rest") as string, provided(argument) as (...(int64, string)) => void) return=void kind=expression target=expression
/// @resolution.call source="each([(1, \"a\")])" parameters=((int64, string)[]) arguments=(provided([(1, "a")]) as (int64, string)[]) return=(string, (...(int64, string)) => void) => void kind=symbol target=each instance="each<(int64, string)>"
/// @generic.instantiation id="each<(int64, string)>" template=each arguments=((int64, string))
/// @resolution.call source=[(1, "a")] parameters=(^Slice<(int64, string)>) arguments=(rest(provided((1, "a")) as (int64, string)) as (int64, string)) return=(int64, string)[] kind=symbol target=arrayFromOwnedSlice instance="arrayFromOwnedSlice<(int64, string)>"
/// @generic.instantiation id="arrayFromOwnedSlice<(int64, string)>" template=arrayFromOwnedSlice arguments=((int64, string))
/// @type.symbol symbol=symbol12 type=Function<(...(int64, string),), void, "readonly">
/// @type.symbol symbol=symbol12.args source=...args type=(int64, string)

    const pack = args;
    /// @type.symbol symbol=symbol12.pack source=pack type=(int64, string)
    /// @resolution.pattern source=pack kind=binding target=symbol12.pack
    /// @resolution.name source=args target=symbol12.args
    /// @resolution.access source=args root=symbol12.args

});

each([(1, "a")])("partial", (count, text: string) => {
/// @resolution.name source=each target=each
/// @resolution.call parameters=(string, (...(int64, string)) => void) arguments=(provided("partial") as string, provided(argument) as (...(int64, string)) => void) return=void kind=expression target=expression
/// @resolution.call source="each([(1, \"a\")])" parameters=((int64, string)[]) arguments=(provided([(1, "a")]) as (int64, string)[]) return=(string, (...(int64, string)) => void) => void kind=symbol target=each instance="each<(int64, string)>"
/// @resolution.call source=[(1, "a")] parameters=(^Slice<(int64, string)>) arguments=(rest(provided((1, "a")) as (int64, string)) as (int64, string)) return=(int64, string)[] kind=symbol target=arrayFromOwnedSlice instance="arrayFromOwnedSlice<(int64, string)>"
/// @type.symbol symbol=symbol15 type=Function<(int64, string), void, "readonly">
/// @type.symbol symbol=symbol15.count source=count type=int64
/// @type.symbol symbol=symbol15.text source="text: string" type=string

    const pair = (count, text);
    /// @type.symbol symbol=symbol15.pair source=pair type=(int64, string)
    /// @resolution.pattern source=pair kind=binding target=symbol15.pair
    /// @resolution.name source=count target=symbol15.count
    /// @resolution.place source=count placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=count root=symbol15.count
    /// @resolution.name source=text target=symbol15.text
    /// @resolution.place source=text placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=text root=symbol15.text

});

each([(1,)])("typed", (count: int32) => {});
/// @resolution.name source=each target=each
/// @resolution.call source="each([(1,)])(\"typed\", (count: int32) => {})" parameters=(string, (...(int32,)) => void) arguments=(provided("typed") as string, provided((count: int32) => {}) as (...(int32,)) => void) return=void kind=expression target=expression
/// @resolution.call source=each([(1,)]) parameters=((int32,)[]) arguments=(provided([(1,)]) as (int32,)[]) return=(string, (...(int32,)) => void) => void kind=symbol target=each instance=each<(int32,)>
/// @generic.instantiation id=each<(int32,)> template=each arguments=((int32,))
/// @resolution.call source=[(1,)] parameters=(^Slice<(int32,)>) arguments=(rest(provided((1,)) as (int32,)) as (int32,)) return=(int32,)[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<(int32,)>
/// @generic.instantiation id=arrayFromOwnedSlice<(int32,)> template=arrayFromOwnedSlice arguments=((int32,))
/// @type.symbol symbol=symbol19 source="(count: int32) => {}" type=Function<(int32,), void, "readonly">
/// @type.symbol symbol=symbol19.count source="count: int32" type=int32

each([(2,)])("typed again", (count: int32) => {});
/// @resolution.name source=each target=each
/// @resolution.call source="each([(2,)])(\"typed again\", (count: int32) => {})" parameters=(string, (...(int32,)) => void) arguments=(provided("typed again") as string, provided((count: int32) => {}) as (...(int32,)) => void) return=void kind=expression target=expression
/// @resolution.call source=each([(2,)]) parameters=((int32,)[]) arguments=(provided([(2,)]) as (int32,)[]) return=(string, (...(int32,)) => void) => void kind=symbol target=each instance=each<(int32,)>
/// @resolution.call source=[(2,)] parameters=(^Slice<(int32,)>) arguments=(rest(provided((2,)) as (int32,)) as (int32,)) return=(int32,)[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<(int32,)>
/// @type.symbol symbol=symbol21 source="(count: int32) => {}" type=Function<(int32,), void, "readonly">
/// @type.symbol symbol=symbol21.count source="count: int32" type=int32
"#,
        r#"

"#,
    );
}

#[test]
fn test_flow_the_declared_result_into_match_and_block_callback_bodies() {
    let session = TestSession::single(
        r#"
function parse(result: Result<string, string>): Result<int32, string> {
    return result.andThen((value) => {
        const parsed = value.isEmpty ? Result.err("empty") : Result.ok(1);
        return parsed;
    });
}

function choose(result: Result<string, string>): Result<int32, string> {
    return result.andThen((value) => match (value.isEmpty) {
        true => Result.err("empty")
        false => Result.ok(1)
    });
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
function parse(result: Result<string, string>): Result<int32, string> {
    return result.andThen<string, string, int32, string>((value: string): Result<int32, string> => {
        const parsed: Result<int32, string> = value.isEmpty
            ? Result.err<int32, string>("empty")
            : Result.ok<int32, string>(1);
        return parsed;
    });
}

function choose(result: Result<string, string>): Result<int32, string> {
    return result.andThen<string, string, int32, string>(
        (value: string): Result<int32, string> =>
            match (value.isEmpty) {
                true => Result.err<int32, string>("empty")
                false => Result.ok<int32, string>(1)
            },
    );
}

=== dir ===
function parse(result: Result<string, string>): Result<int32, string> {
/// @type.symbol symbol=parse type=(Result<string, string>) => Result<int32, string>
/// @type.symbol symbol=parse.result source="result: Result<string, string>" type=Result<string, string>
/// @resolution.name source=Result target=Result
/// @resolution.name source=Result target=Result

    return result.andThen((value) => {
    /// @resolution.name source=result target=parse.result
    /// @resolution.member source=result.andThen receiver=Result<string, string> type=<andThen.U, andThen.F>(this: Result<string, string>, (string) => Result<andThen.U, andThen.F>) => Result<andThen.U, string | andThen.F> kind=symbol target_receiver=Result<string, string> target=andThen
    /// @resolution.call parameters=((string) => Result<int32, string>) arguments=(provided(argument) as (string) => Result<int32, string>) return=Result<int32, string> kind=symbol target=andThen receiver=Result<string, string> instance="Result<string, string>.<extension#1>.andThen<int32, string>"
    /// @resolution.place source=result placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=result root=parse.result
    /// @generic.instantiation id="andThen<string, string, int32, string>" template=andThen arguments=(string, string, int32, string)
    /// @generic.instantiation id="andThen<string, string>" template=andThen arguments=(string, string)
    /// @type.symbol symbol=parse.symbol3 type=Function<(string,), Result<int32, string>, "readonly">
    /// @type.symbol symbol=parse.symbol3.value source=value type=string

        const parsed = value.isEmpty ? Result.err("empty") : Result.ok(1);
        /// @type.symbol symbol=parse.symbol3.parsed source=parsed type=Result<int32, string>
        /// @resolution.pattern source=parsed kind=binding target=parse.symbol3.parsed
        /// @resolution.name source=value target=parse.symbol3.value
        /// @resolution.member source=value.isEmpty receiver=string type=boolean kind=call target="isEmpty(parameters=(), arguments=(), return=boolean, regions=(\"managed\" & \"local\"))"
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=parse.symbol3.value
        /// @generic.instantiation id="isEmpty<\"managed\" & \"local\">" template=isEmpty arguments=("managed" & "local")
        /// @resolution.name source=Result target=Result
        /// @resolution.member source=Result.err receiver=Result type=(E#1) => Result<T#1, E#1> kind=symbol target_receiver=Result target=err#1
        /// @resolution.call source="Result.err(\"empty\")" parameters=(string) arguments=(provided("empty") as string) return=Result<int32, string> kind=symbol target=err#1 instance="Result<int32, string>.<extension#1>.err#1"
        /// @generic.instantiation id="err#1<int32, string>" template=err#1 arguments=(int32, string)
        /// @resolution.name source=Result target=Result
        /// @resolution.member source=Result.ok receiver=Result type=(T#1) => Result<T#1, E#1> kind=symbol target_receiver=Result target=ok#1
        /// @resolution.call source=Result.ok(1) parameters=(int32) arguments=(provided(1) as int32) return=Result<int32, string> kind=symbol target=ok#1 instance="Result<int32, string>.<extension#1>.ok#1"
        /// @generic.instantiation id="ok#1<int32, string>" template=ok#1 arguments=(int32, string)

        return parsed;
        /// @resolution.name source=parsed target=parse.symbol3.parsed
        /// @resolution.place source=parsed placement="local" lifetime="frame" access="immutable"
        /// @resolution.access source=parsed root=parse.symbol3.parsed

    });
}

function choose(result: Result<string, string>): Result<int32, string> {
/// @type.symbol symbol=choose type=(Result<string, string>) => Result<int32, string>
/// @type.symbol symbol=choose.result source="result: Result<string, string>" type=Result<string, string>
/// @resolution.name source=Result target=Result
/// @resolution.name source=Result target=Result

    return result.andThen((value) => match (value.isEmpty) {
    /// @resolution.name source=result target=choose.result
    /// @resolution.member source=result.andThen receiver=Result<string, string> type=<andThen.U, andThen.F>(this: Result<string, string>, (string) => Result<andThen.U, andThen.F>) => Result<andThen.U, string | andThen.F> kind=symbol target_receiver=Result<string, string> target=andThen
    /// @resolution.call parameters=((string) => Result<int32, string>) arguments=(provided(argument) as (string) => Result<int32, string>) return=Result<int32, string> kind=symbol target=andThen receiver=Result<string, string> instance="Result<string, string>.<extension#1>.andThen<int32, string>"
    /// @resolution.place source=result placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=result root=choose.result
    /// @type.symbol symbol=choose.symbol8 type=Function<(string,), Result<int32, string>, "readonly">
    /// @type.symbol symbol=choose.symbol8.value source=value type=string
    /// @resolution.coverage exhaustive=true disjoint=true
    /// @resolution.name source=value target=choose.symbol8.value
    /// @resolution.member source=value.isEmpty receiver=string type=boolean kind=call target="isEmpty(parameters=(), arguments=(), return=boolean, regions=(\"managed\" & \"local\"))"
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=choose.symbol8.value

        true => Result.err("empty")
        /// @resolution.pattern source=true kind=literal value=true
        /// @resolution.name source=Result target=Result
        /// @resolution.member source=Result.err receiver=Result type=(E#1) => Result<T#1, E#1> kind=symbol target_receiver=Result target=err#1
        /// @resolution.call source="Result.err(\"empty\")" parameters=(string) arguments=(provided("empty") as string) return=Result<int32, string> kind=symbol target=err#1 instance="Result<int32, string>.<extension#1>.err#1"

        false => Result.ok(1)
        /// @resolution.pattern source=false kind=literal value=false
        /// @resolution.name source=Result target=Result
        /// @resolution.member source=Result.ok receiver=Result type=(T#1) => Result<T#1, E#1> kind=symbol target_receiver=Result target=ok#1
        /// @resolution.call source=Result.ok(1) parameters=(int32) arguments=(provided(1) as int32) return=Result<int32, string> kind=symbol target=ok#1 instance="Result<int32, string>.<extension#1>.ok#1"

    });
}
"#,
        r#"

"#,
    );
}
