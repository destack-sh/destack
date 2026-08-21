use crate::tests::{DirRows, TestSession};

#[test]
fn test_find_an_owner_member_on_a_narrowed_variant() {
    let session = TestSession::single(
        r#"
import { Result } from "destack:error";

declare const result: Result<int32, string>;

function run(): int32 {
    if (result.isOk()) {
        return result.unwrap();
    }
    return result.unwrapOr(0);
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Result } from "destack:error";

declare const result: Result<int32, string>;

function run(): int32 {
    if (result.isOk<int32, string>()) {
        return result.unwrap<int32, string>();
    }
    return result.unwrapOr<int32, string>(0);
}

=== dir ===
import { Result } from "destack:error";

declare const result: Result<int32, string>;
/// @type.symbol symbol=result source=result type=error.result.Result<int32, string>
/// @resolution.pattern source=result kind=binding target=result
/// @resolution.name source=Result target=error.result.Result

function run(): int32 {
/// @type.symbol symbol=run type=() => int32

    if (result.isOk()) {
    /// @resolution.name source=result target=result
    /// @resolution.member source=result.isOk receiver=error.result.Result<int32, string> type=<error.result.isOk.'a>(this: &error.result.isOk.'a readonly error.result.Result<int32, string>) => boolean kind=symbol target_receiver=error.result.Result<int32, string> target=error.result.isOk
    /// @resolution.call source=result.isOk() parameters=() return=boolean kind=symbol target=error.result.isOk receiver=error.result.Result<int32, string> adjustments=(borrow(&'static readonly error.result.Result<int32, string>)) instance="error.result.Result<int32, string>.<extension#1>.isOk"
    /// @resolution.place source=result placement="local" lifetime="static" access="readonly"
    /// @resolution.access source=result root=result
    /// @generic.instantiation id="error.result.isOk<int32, string>" template=error.result.isOk arguments=(int32, string)

        return result.unwrap();
        /// @resolution.name source=result target=result
        /// @resolution.member source=result.unwrap receiver=error.result.Result<int32, string> type=(this: error.result.Result<int32, string>) => int32 kind=symbol target_receiver=error.result.Result<int32, string> target=error.result.unwrap
        /// @resolution.call source=result.unwrap() parameters=() return=int32 kind=symbol target=error.result.unwrap receiver=error.result.Result<int32, string> instance="error.result.Result<int32, string>.<extension#1>.unwrap"
        /// @resolution.place source=result placement="local" lifetime="static" access="readonly"
        /// @resolution.access source=result root=result
        /// @generic.instantiation id="error.result.unwrap<int32, string>" template=error.result.unwrap arguments=(int32, string)

    }
    return result.unwrapOr(0);
    /// @resolution.name source=result target=result
    /// @resolution.member source=result.unwrapOr receiver=error.result.Result<int32, string> type=(this: error.result.Result<int32, string>, int32) => int32 kind=symbol target_receiver=error.result.Result<int32, string> target=error.result.unwrapOr
    /// @resolution.call source=result.unwrapOr(0) parameters=(int32) arguments=(provided(0) as int32) return=int32 kind=symbol target=error.result.unwrapOr receiver=error.result.Result<int32, string> instance="error.result.Result<int32, string>.<extension#1>.unwrapOr"
    /// @resolution.place source=result placement="local" lifetime="static" access="readonly"
    /// @resolution.access source=result root=result
    /// @generic.instantiation id="error.result.unwrapOr<int32, string>" template=error.result.unwrapOr arguments=(int32, string)

}
"#,
        r#"
"#,
    );
}

#[test]
fn test_find_a_declared_type_member_on_a_narrowed_binding() {
    let session = TestSession::single(
        r#"
class Base {
    shared(): int32 {
        return 1;
    }
}

class Derived extends Base {
    extra(): int32 {
        return 2;
    }
}

function run(value: Base): int32 {
    if (value instanceof Derived) {
        return value.extra() + value.shared();
    }
    return value.shared();
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Base {
    shared(): int32 {
        return 1;
    }
}

class Derived extends Base {
    extra(): int32 {
        return 2;
    }
}

function run(value: Base): int32 {
    if (value instanceof Derived) {
        return value.extra() + value.shared();
    }
    return value.shared();
}

=== dir ===
class Base {
/// @type.symbol symbol=Base type=Base
/// @definition.class symbol=Base
/// @definition.method symbol=Base.shared slot=shared type=(this: this) => int32

    shared(): int32 {
    /// @type.symbol symbol=Base.shared type=(this: this) => int32

        return 1;
    }
}

class Derived extends Base {
/// @type.symbol symbol=Derived type=Derived
/// @definition.class symbol=Derived
/// @definition.extends symbol=Derived source=Base target=Base
/// @definition.method symbol=Derived.extra slot=extra type=(this: this) => int32
/// @resolution.name source=Base target=Base

    extra(): int32 {
    /// @type.symbol symbol=Derived.extra type=(this: this) => int32

        return 2;
    }
}

function run(value: Base): int32 {
/// @type.symbol symbol=run type=(Base) => int32
/// @type.symbol symbol=run.value source="value: Base" type=Base
/// @resolution.name source=Base target=Base

    if (value instanceof Derived) {
    /// @resolution.name source=value target=run.value
    /// @resolution.guard source="value instanceof Derived" kind=instanceof value=Base target=Derived target_type=Derived predicate="Base is subtype(Derived)" narrowed=Narrow<Base, Derived>
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=run.value
    /// @resolution.name source=Derived target=Derived

        return value.extra() + value.shared();
        /// @resolution.name source=value target=run.value
        /// @resolution.member source=value.extra receiver=Narrow<Base, Derived> type=(this: Derived) => int32 kind=symbol target_receiver=Narrow<Base, Derived> target=Derived.extra
        /// @resolution.call source=value.extra() parameters=() return=int32 kind=symbol target=Derived.extra receiver=Narrow<Base, Derived>
        /// @resolution.operator source="value.extra() + value.shared()" type=int32 operator="+" kind=builtin operands=[value.extra() as int32 families=(integer), value.shared() as int32 families=(integer)]
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=run.value
        /// @resolution.name source=value target=run.value
        /// @resolution.member source=value.shared receiver=Narrow<Base, Derived> type=(this: Derived) => int32 kind=symbol target_receiver=Narrow<Base, Derived> target=Base.shared
        /// @resolution.call source=value.shared() parameters=() return=int32 kind=symbol target=Base.shared receiver=Narrow<Base, Derived>
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=run.value

    }
    return value.shared();
    /// @resolution.name source=value target=run.value
    /// @resolution.member source=value.shared receiver=Base type=(this: Base) => int32 kind=symbol target_receiver=Base target=Base.shared
    /// @resolution.call source=value.shared() parameters=() return=int32 kind=symbol target=Base.shared receiver=Base
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=run.value

}
"#,
        r#"
"#,
    );
}

#[test]
fn test_read_the_narrowed_result_payload_after_a_kind_check() {
    let session = TestSession::single(
        r#"
function value(result: Result<int32, string>): int32 {
    if (result.kind === "Ok") {
        return result.value;
    }
    return result.unwrap();
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function value(result: Result<int32, string>): int32 {
    if (result.kind === ("Ok" as "Ok" | "Err")) {
        return result.value;
    }
    return result.unwrap<int32, string>();
}

=== dir ===
function value(result: Result<int32, string>): int32 {
/// @type.symbol symbol=value type=(Result<int32, string>) => int32
/// @type.symbol symbol=value.result source="result: Result<int32, string>" type=Result<int32, string>
/// @resolution.name source=Result target=error.result.Result

    if (result.kind === "Ok") {
    /// @resolution.name source=result target=value.result
    /// @resolution.member source=result.kind receiver=Result<int32, string> type="Ok" | "Err" kind=projection target="discriminant(error.result.Ok<int32> | error.result.Err<string>, kind, cases=[error.result.Ok<int32>: Ok, error.result.Err<string>: Err], \"Ok\" | \"Err\")" adjustments=(newtype.payload(error.result.Result, error.result.Ok<int32> | error.result.Err<string>))
    /// @resolution.operator source="result.kind === \"Ok\"" type=boolean operator="===" kind=builtin operands=[result.kind as "Ok" | "Err" families=(string), "Ok" as "Ok" | "Err" families=(string)]
    /// @resolution.place source=result placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=result root=value.result
    /// @resolution.place source=result.kind placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=result.kind root=value.result keys=[kind]
    /// @generic.instantiation id="Result<int32, string>" template=error.result.Result arguments=(int32, string)

        return result.value;
        /// @resolution.name source=result target=value.result
        /// @resolution.member source=result.value receiver=Result<int32, string> & error.result.Ok<int32> type=int32 kind=field target_receiver=Result<int32, string> & error.result.Ok<int32> adjustments=(newtype.payload(error.result.Result, error.result.Ok<int32> | error.result.Err<string>), union.payload(error.result.Ok<int32> | error.result.Err<string>, error.result.Ok<int32>, error.result.Ok<int32>)) key=value target=error.result.Ok.value target_type=int32
        /// @resolution.place source=result placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=result root=value.result
        /// @resolution.place source=result.value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=result.value root=value.result keys=[value]

    }
    return result.unwrap();
    /// @resolution.name source=result target=value.result
    /// @resolution.member source=result.unwrap receiver=Result<int32, string> & error.result.Err<string> type=(this: Result<int32, string>) => int32 kind=symbol target_receiver=Result<int32, string> & error.result.Err<string> target=error.result.unwrap
    /// @resolution.call source=result.unwrap() parameters=() return=int32 kind=symbol target=error.result.unwrap receiver=Result<int32, string> & error.result.Err<string> instance="Result<int32, string>.<extension#1>.unwrap"
    /// @resolution.place source=result placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=result root=value.result
    /// @generic.instantiation id="error.result.unwrap<int32, string>" template=error.result.unwrap arguments=(int32, string)

}
"#,
        r#"

"#,
    );
}

#[test]
fn test_read_declared_union_methods_through_narrowed_arms_and_calls() {
    let session = TestSession::single(
        r#"
function describe(result: Result<int32, string>): int32 {
    if (result.kind === "Err") {
        return result.unwrap();
    }
    return result.unwrap() + 1;
}

function pick(values: Array<Result<int32, string>>): int32 {
    const first = values[0];
    if (first.kind === "Ok") {
        return first.unwrap();
    }
    return 0;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function describe(result: Result<int32, string>): int32 {
    if (result.kind === ("Err" as "Ok" | "Err")) {
        return result.unwrap<int32, string>();
    }
    return result.unwrap<int32, string>() + 1;
}

function pick(values: Result<int32, string>[]): int32 {
    const first: Result<int32, string> = values[0];
    if (first.kind === ("Ok" as "Ok" | "Err")) {
        return first.unwrap<int32, string>();
    }
    return 0;
}

=== dir ===
function describe(result: Result<int32, string>): int32 {
/// @type.symbol symbol=describe type=(Result<int32, string>) => int32
/// @type.symbol symbol=describe.result source="result: Result<int32, string>" type=Result<int32, string>
/// @resolution.name source=Result target=error.result.Result

    if (result.kind === "Err") {
    /// @resolution.name source=result target=describe.result
    /// @resolution.member source=result.kind receiver=Result<int32, string> type="Ok" | "Err" kind=projection target="discriminant(error.result.Ok<int32> | error.result.Err<string>, kind, cases=[error.result.Ok<int32>: Ok, error.result.Err<string>: Err], \"Ok\" | \"Err\")" adjustments=(newtype.payload(error.result.Result, error.result.Ok<int32> | error.result.Err<string>))
    /// @resolution.operator source="result.kind === \"Err\"" type=boolean operator="===" kind=builtin operands=[result.kind as "Ok" | "Err" families=(string), "Err" as "Ok" | "Err" families=(string)]
    /// @resolution.place source=result placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=result root=describe.result
    /// @resolution.place source=result.kind placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=result.kind root=describe.result keys=[kind]
    /// @generic.instantiation id="Result<int32, string>" template=error.result.Result arguments=(int32, string)

        return result.unwrap();
        /// @resolution.name source=result target=describe.result
        /// @resolution.member source=result.unwrap receiver=Result<int32, string> & error.result.Err<string> type=(this: Result<int32, string>) => int32 kind=symbol target_receiver=Result<int32, string> & error.result.Err<string> target=error.result.unwrap
        /// @resolution.call source=result.unwrap() parameters=() return=int32 kind=symbol target=error.result.unwrap receiver=Result<int32, string> & error.result.Err<string> instance="Result<int32, string>.<extension#1>.unwrap"
        /// @resolution.place source=result placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=result root=describe.result
        /// @generic.instantiation id="error.result.unwrap<int32, string>" template=error.result.unwrap arguments=(int32, string)

    }
    return result.unwrap() + 1;
    /// @resolution.name source=result target=describe.result
    /// @resolution.member source=result.unwrap receiver=Result<int32, string> & error.result.Ok<int32> type=(this: Result<int32, string>) => int32 kind=symbol target_receiver=Result<int32, string> & error.result.Ok<int32> target=error.result.unwrap
    /// @resolution.call source=result.unwrap() parameters=() return=int32 kind=symbol target=error.result.unwrap receiver=Result<int32, string> & error.result.Ok<int32> instance="Result<int32, string>.<extension#1>.unwrap"
    /// @resolution.operator source="result.unwrap() + 1" type=int32 operator="+" kind=builtin operands=[result.unwrap() as int32 families=(integer), 1 as int32 families=(integer)]
    /// @resolution.place source=result placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=result root=describe.result

}

function pick(values: Array<Result<int32, string>>): int32 {
/// @type.symbol symbol=pick type=(Result<int32, string>[]) => int32
/// @type.symbol symbol=pick.values source="values: Array<Result<int32, string>>" type=Result<int32, string>[]
/// @resolution.name source=Array target=collections.array.Array
/// @resolution.name source=Result target=error.result.Result

    const first = values[0];
    /// @type.symbol symbol=pick.first source=first type=Result<int32, string>
    /// @resolution.pattern source=first kind=binding target=pick.first
    /// @resolution.name source=values target=pick.values
    /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values root=pick.values
    /// @resolution.access source=values[0] root=pick.values keys=[0]
    /// @resolution.subscript source=values[0] type=Result<int32, string> kind=call target="collections.array.index#1(parameters=(isize), arguments=(provided(0) as isize), return=memory.type.WithAccess<&'frame Result<int32, string>, \"exclusive\">)"
    /// @generic.instantiation id="collections.array.index#1<Result<int32, string>, \"exclusive\">" template=collections.array.index#1 arguments=(Result<int32, string>, "exclusive")

    if (first.kind === "Ok") {
    /// @resolution.name source=first target=pick.first
    /// @resolution.member source=first.kind receiver=Result<int32, string> type="Ok" | "Err" kind=projection target="discriminant(error.result.Ok<int32> | error.result.Err<string>, kind, cases=[error.result.Ok<int32>: Ok, error.result.Err<string>: Err], \"Ok\" | \"Err\")" adjustments=(newtype.payload(error.result.Result, error.result.Ok<int32> | error.result.Err<string>))
    /// @resolution.operator source="first.kind === \"Ok\"" type=boolean operator="===" kind=builtin operands=[first.kind as "Ok" | "Err" families=(string), "Ok" as "Ok" | "Err" families=(string)]
    /// @resolution.place source=first placement="local" lifetime="frame" access="readonly"
    /// @resolution.access source=first root=pick.first
    /// @resolution.place source=first.kind placement="local" lifetime="frame" access="readonly"
    /// @resolution.access source=first.kind root=pick.first keys=[kind]

        return first.unwrap();
        /// @resolution.name source=first target=pick.first
        /// @resolution.member source=first.unwrap receiver=Result<int32, string> & error.result.Ok<int32> type=(this: Result<int32, string>) => int32 kind=symbol target_receiver=Result<int32, string> & error.result.Ok<int32> target=error.result.unwrap
        /// @resolution.call source=first.unwrap() parameters=() return=int32 kind=symbol target=error.result.unwrap receiver=Result<int32, string> & error.result.Ok<int32> instance="Result<int32, string>.<extension#1>.unwrap"
        /// @resolution.place source=first placement="local" lifetime="frame" access="readonly"
        /// @resolution.access source=first root=pick.first

    }
    return 0;
}
"#,
        r#"

"#,
    );
}
