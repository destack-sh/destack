use crate::tests::{DirRows, TestSession};

/// A method of the owning union resolves on a narrowed variant.
#[test]
fn test_find_an_owner_member_on_a_narrowed_variant() {
    let session = TestSession::single(
        r#"
import { Result } from "tspp:error";

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
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { Result } from "tspp:error";

declare const result: Result<int32, string>;

function run(): int32 {
    if (result.isOk<int32, string, "static">()) {
        return result.unwrap<int32, string>();
    }
    return result.unwrapOr<int32, string>(0);
}

=== dir ===
import { Result } from "tspp:error";

declare const result: Result<int32, string>;
/// @type.symbol symbol=result source=result type=Result<int32, string>
/// @resolution.pattern source=result kind=binding target=result
/// @resolution.name source=Result target=Result

function run(): int32 {
/// @type.symbol symbol=run type=() => int32

    if (result.isOk()) {
    /// @resolution.name source=result target=result
    /// @resolution.member source=result.isOk receiver=Result<int32, string> type=<isOk.'a>(this: &isOk.'a readonly Result<int32, string>) => boolean kind=symbol target_receiver=Result<int32, string> target=isOk
    /// @resolution.call source=result.isOk() parameters=() return=boolean regions=("static" & "local") kind=symbol target=isOk receiver=Result<int32, string> adjustments=(borrow(&'static readonly Result<int32, string>)) instance="Result<int32, string>.<extension#1>.isOk<\"static\" & \"local\">"
    /// @resolution.place source=result placement="local" lifetime="static" access="immutable"
    /// @resolution.access source=result root=result
    /// @generic.instantiation id="isOk<int32, string, \"static\" & \"local\">" template=isOk arguments=(int32, string, "static" & "local")
    /// @generic.instantiation id="isOk<int32, string>" template=isOk arguments=(int32, string)

        return result.unwrap();
        /// @resolution.name source=result target=result
        /// @resolution.member source=result.unwrap receiver=Result<int32, string> type=(this: Result<int32, string>) => int32 kind=symbol target_receiver=Result<int32, string> target=unwrap
        /// @resolution.call source=result.unwrap() parameters=() return=int32 kind=symbol target=unwrap receiver=Result<int32, string> instance="Result<int32, string>.<extension#1>.unwrap"
        /// @resolution.place source=result placement="local" lifetime="static" access="immutable"
        /// @resolution.access source=result root=result
        /// @generic.instantiation id="unwrap<int32, string>" template=unwrap arguments=(int32, string)

    }
    return result.unwrapOr(0);
    /// @resolution.name source=result target=result
    /// @resolution.member source=result.unwrapOr receiver=Result<int32, string> type=(this: Result<int32, string>, int32) => int32 kind=symbol target_receiver=Result<int32, string> target=unwrapOr
    /// @resolution.call source=result.unwrapOr(0) parameters=(int32) arguments=(provided(0) as int32) return=int32 kind=symbol target=unwrapOr receiver=Result<int32, string> instance="Result<int32, string>.<extension#1>.unwrapOr"
    /// @resolution.place source=result placement="local" lifetime="static" access="immutable"
    /// @resolution.access source=result root=result
    /// @generic.instantiation id="unwrapOr<int32, string>" template=unwrapOr arguments=(int32, string)

}
"#,
        r#"
"#,
    );
}

/// A method declared on a subclass resolves on a binding narrowed by instanceof.
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
        "main.tspp",
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
/// @type.symbol symbol=Base type=typeof Base
/// @definition.class symbol=Base
/// @definition.method symbol=Base.shared slot=shared type=(this: Base) => int32

    shared(): int32 {
    /// @type.symbol symbol=Base.shared type=(this: Base) => int32
    /// @type.symbol symbol=Base.shared.this type=Base

        return 1;
    }
}

class Derived extends Base {
/// @type.symbol symbol=Derived type=typeof Derived
/// @definition.class symbol=Derived
/// @definition.extends symbol=Derived source=Base target=Base
/// @definition.method symbol=Derived.extra slot=extra type=(this: Derived) => int32
/// @resolution.name source=Base target=Base

    extra(): int32 {
    /// @type.symbol symbol=Derived.extra type=(this: Derived) => int32
    /// @type.symbol symbol=Derived.extra.this type=Derived

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
        /// @resolution.member source=value.shared receiver=Narrow<Base, Derived> type=(this: Base) => int32 kind=symbol target_receiver=Narrow<Base, Derived> target=Base.shared
        /// @resolution.call source=value.shared() parameters=() return=int32 kind=symbol target=Base.shared receiver=Narrow<Base, Derived> adjustments=(upcast(Base))
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

/// A kind check narrows a result and exposes the payload field of its arm.
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
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
function value(result: Result<int32, string>): int32 {
    if (result.kind === "Ok") {
        return result.value;
    }
    return result.unwrap<int32, string>();
}

=== dir ===
function value(result: Result<int32, string>): int32 {
/// @type.symbol symbol=value type=(Result<int32, string>) => int32
/// @type.symbol symbol=value.result source="result: Result<int32, string>" type=Result<int32, string>
/// @resolution.name source=Result target=Result

    if (result.kind === "Ok") {
    /// @resolution.name source=result target=value.result
    /// @resolution.member source=result.kind receiver=Result<int32, string> type="Ok" | "Err" kind=projection target="discriminant(Ok<int32> | Err<string>, kind, cases=[Ok<int32>: Ok, Err<string>: Err], \"Ok\" | \"Err\")" adjustments=(newtype.payload(Result, Ok<int32> | Err<string>))
    /// @resolution.operator source="result.kind === \"Ok\"" type=boolean operator="===" kind=builtin operands=[result.kind as "Ok" | "Err" families=(string), "Ok" as "Ok" | "Err" families=(string)]
    /// @resolution.place source=result placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=result root=value.result
    /// @resolution.place source=result.kind placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=result.kind root=value.result keys=[kind]
    /// @generic.instantiation id="Result<int32, string>" template=Result arguments=(int32, string)

        return result.value;
        /// @resolution.name source=result target=value.result
        /// @resolution.member source=result.value receiver=Result<int32, string> & Ok<int32> type=int32 kind=field target_receiver=Result<int32, string> & Ok<int32> adjustments=(newtype.payload(Result, Ok<int32> | Err<string>), union.payload(Ok<int32> | Err<string>, Ok<int32>, Ok<int32>)) key=value target=Ok.value target_type=int32
        /// @resolution.place source=result placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=result root=value.result
        /// @resolution.place source=result.value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=result.value root=value.result keys=[value]

    }
    return result.unwrap();
    /// @resolution.name source=result target=value.result
    /// @resolution.member source=result.unwrap receiver=Result<int32, string> & Err<string> type=(this: Result<int32, string>) => int32 kind=symbol target_receiver=Result<int32, string> & Err<string> target=unwrap
    /// @resolution.call source=result.unwrap() parameters=() return=int32 kind=symbol target=unwrap receiver=Result<int32, string> & Err<string> instance="Result<int32, string>.<extension#1>.unwrap"
    /// @resolution.place source=result placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=result root=value.result
    /// @generic.instantiation id="unwrap<int32, string>" template=unwrap arguments=(int32, string)

}
"#,
        r#"

"#,
    );
}

/// Methods of the union resolve on arms narrowed from bindings and from index reads.
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
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
function describe(result: Result<int32, string>): int32 {
    if (result.kind === "Err") {
        return result.unwrap<int32, string>();
    }
    return result.unwrap<int32, string>() + 1;
}

function pick(values: Result<int32, string>[]): int32 {
    const first: Result<int32, string> = values[0];
    if (first.kind === "Ok") {
        return first.unwrap<int32, string>();
    }
    return 0;
}

=== dir ===
function describe(result: Result<int32, string>): int32 {
/// @type.symbol symbol=describe type=(Result<int32, string>) => int32
/// @type.symbol symbol=describe.result source="result: Result<int32, string>" type=Result<int32, string>
/// @resolution.name source=Result target=Result

    if (result.kind === "Err") {
    /// @resolution.name source=result target=describe.result
    /// @resolution.member source=result.kind receiver=Result<int32, string> type="Ok" | "Err" kind=projection target="discriminant(Ok<int32> | Err<string>, kind, cases=[Ok<int32>: Ok, Err<string>: Err], \"Ok\" | \"Err\")" adjustments=(newtype.payload(Result, Ok<int32> | Err<string>))
    /// @resolution.operator source="result.kind === \"Err\"" type=boolean operator="===" kind=builtin operands=[result.kind as "Ok" | "Err" families=(string), "Err" as "Ok" | "Err" families=(string)]
    /// @resolution.place source=result placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=result root=describe.result
    /// @resolution.place source=result.kind placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=result.kind root=describe.result keys=[kind]
    /// @generic.instantiation id="Result<int32, string>" template=Result arguments=(int32, string)

        return result.unwrap();
        /// @resolution.name source=result target=describe.result
        /// @resolution.member source=result.unwrap receiver=Result<int32, string> & Err<string> type=(this: Result<int32, string>) => int32 kind=symbol target_receiver=Result<int32, string> & Err<string> target=unwrap
        /// @resolution.call source=result.unwrap() parameters=() return=int32 kind=symbol target=unwrap receiver=Result<int32, string> & Err<string> instance="Result<int32, string>.<extension#1>.unwrap"
        /// @resolution.place source=result placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=result root=describe.result
        /// @generic.instantiation id="unwrap<int32, string>" template=unwrap arguments=(int32, string)

    }
    return result.unwrap() + 1;
    /// @resolution.name source=result target=describe.result
    /// @resolution.member source=result.unwrap receiver=Result<int32, string> & Ok<int32> type=(this: Result<int32, string>) => int32 kind=symbol target_receiver=Result<int32, string> & Ok<int32> target=unwrap
    /// @resolution.call source=result.unwrap() parameters=() return=int32 kind=symbol target=unwrap receiver=Result<int32, string> & Ok<int32> instance="Result<int32, string>.<extension#1>.unwrap"
    /// @resolution.operator source="result.unwrap() + 1" type=int32 operator="+" kind=builtin operands=[result.unwrap() as int32 families=(integer), 1 as int32 families=(integer)]
    /// @resolution.place source=result placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=result root=describe.result

}

function pick(values: Array<Result<int32, string>>): int32 {
/// @type.symbol symbol=pick type=(Result<int32, string>[]) => int32
/// @type.symbol symbol=pick.values source="values: Array<Result<int32, string>>" type=Result<int32, string>[]
/// @resolution.name source=Array target=Array
/// @resolution.name source=Result target=Result

    const first = values[0];
    /// @type.symbol symbol=pick.first source=first type=Result<int32, string>
    /// @resolution.pattern source=first kind=binding target=pick.first
    /// @resolution.name source=values target=pick.values
    /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values root=pick.values
    /// @resolution.subscript source=values[0] type=Result<int32, string> kind=call target="index#2(parameters=(isize), arguments=(provided(0) as isize), return=Result<int32, string>, regions=(\"managed\" & \"local\"))"
    /// @generic.instantiation id="index#2<Result<int32, string>, \"managed\" & \"local\">" template=index#2 arguments=(Result<int32, string>, "managed" & "local")

    if (first.kind === "Ok") {
    /// @resolution.name source=first target=pick.first
    /// @resolution.member source=first.kind receiver=Result<int32, string> type="Ok" | "Err" kind=projection target="discriminant(Ok<int32> | Err<string>, kind, cases=[Ok<int32>: Ok, Err<string>: Err], \"Ok\" | \"Err\")" adjustments=(newtype.payload(Result, Ok<int32> | Err<string>))
    /// @resolution.operator source="first.kind === \"Ok\"" type=boolean operator="===" kind=builtin operands=[first.kind as "Ok" | "Err" families=(string), "Ok" as "Ok" | "Err" families=(string)]
    /// @resolution.place source=first placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=first root=pick.first
    /// @resolution.place source=first.kind placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=first.kind root=pick.first keys=[kind]

        return first.unwrap();
        /// @resolution.name source=first target=pick.first
        /// @resolution.member source=first.unwrap receiver=Result<int32, string> & Ok<int32> type=(this: Result<int32, string>) => int32 kind=symbol target_receiver=Result<int32, string> & Ok<int32> target=unwrap
        /// @resolution.call source=first.unwrap() parameters=() return=int32 kind=symbol target=unwrap receiver=Result<int32, string> & Ok<int32> instance="Result<int32, string>.<extension#1>.unwrap"
        /// @resolution.place source=first placement="local" lifetime="frame" access="immutable"
        /// @resolution.access source=first root=pick.first

    }
    return 0;
}
"#,
        r#"

"#,
    );
}
