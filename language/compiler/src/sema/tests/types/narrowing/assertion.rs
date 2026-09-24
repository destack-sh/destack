use crate::tests::{DirRows, TestSession};

/// Narrow by equality against a singleton widened behind an `as` assertion.
#[test]
fn test_equality_narrows_through_an_as_assertion() {
    let session = TestSession::single(
        r#"
function first(value: int32 | undefined): int32 {
    if (value !== (undefined as int32 | undefined)) {
        return value;
    }
    return 0;
}
"#,
    );

    session.assert_dir("main.ds", DirRows::checked().with_node_types(), r#"
=== annotated ===
function first(value: int32 | undefined): int32 {
    if (value !== (undefined as int32 | undefined)) {
        return value;
    }
    return 0;
}

=== dir ===
function first(value: int32 | undefined): int32 {
/// @type.symbol symbol=first type=(int32 | undefined) => int32
/// @type.symbol symbol=first.value source="value: int32 | undefined" type=int32 | undefined

    if (value !== (undefined as int32 | undefined)) {
    /// @type.node source="value !== (undefined as int32 | undefined)" type=boolean
    /// @resolution.name source=value target=first.value
    /// @resolution.operator source="value !== (undefined as int32 | undefined)" type=boolean operator="!==" kind=builtin operands=[value as int32 | undefined families=(integer | undefined), undefined as int32 | undefined as int32 | undefined families=(integer | undefined)]
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=first.value
    /// @type.node source="undefined as int32 | undefined" type=int32 | undefined
    /// @type.node source=undefined type=undefined

        return value;
        /// @resolution.name source=value target=first.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=first.value
        /// @resolution.narrowing source=value union=int32 | undefined arms=int32

    }
    return 0;
    /// @type.node source=0 type=0

}
"#);
}

/// Narrow by equality against a singleton kept by a `satisfies` assertion.
#[test]
fn test_equality_narrows_through_a_satisfies_assertion() {
    let session = TestSession::single(
        r#"
function first(value: int32 | undefined): int32 {
    if (value !== (undefined satisfies int32 | undefined)) {
        return value;
    }
    return 0;
}
"#,
    );

    session.assert_dir("main.ds", DirRows::checked().with_node_types(), r#"
=== annotated ===
function first(value: int32 | undefined): int32 {
    if (value !== ((undefined satisfies int32 | undefined) as int32 | undefined)) {
        return value;
    }
    return 0;
}

=== dir ===
function first(value: int32 | undefined): int32 {
/// @type.symbol symbol=first type=(int32 | undefined) => int32
/// @type.symbol symbol=first.value source="value: int32 | undefined" type=int32 | undefined

    if (value !== (undefined satisfies int32 | undefined)) {
    /// @type.node source="value !== (undefined satisfies int32 | undefined)" type=boolean
    /// @resolution.name source=value target=first.value
    /// @resolution.operator source="value !== (undefined satisfies int32 | undefined)" type=boolean operator="!==" kind=builtin operands=[value as int32 | undefined families=(integer | undefined), undefined satisfies int32 | undefined as int32 | undefined families=(integer | undefined)]
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=first.value
    /// @type.node source="undefined satisfies int32 | undefined" type=undefined
    /// @type.node source=undefined type=undefined

        return value;
        /// @resolution.name source=value target=first.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=first.value
        /// @resolution.narrowing source=value union=int32 | undefined arms=int32

    }
    return 0;
    /// @type.node source=0 type=0

}
"#);
}

/// Keep the flow unnarrowed when the asserted comparand is no singleton.
#[test]
fn test_equality_ignores_a_widened_non_singleton_assertion() {
    let session = TestSession::single(
        r#"
function keep(value: int32 | undefined, other: int32 | undefined): int32 {
    if (value !== (other as int32 | undefined)) {
        return value;
    }
    return 0;
}
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
function keep(value: int32 | undefined, other: int32 | undefined): int32 {
    if (value !== (other as int32 | undefined)) {
        return value;
    }
    return 0;
}

=== dir ===
function keep(value: int32 | undefined, other: int32 | undefined): int32 {
/// @type.symbol symbol=keep type=(int32 | undefined, int32 | undefined) => int32
/// @type.symbol symbol=keep.value source="value: int32 | undefined" type=int32 | undefined
/// @type.symbol symbol=keep.other source="other: int32 | undefined" type=int32 | undefined

    if (value !== (other as int32 | undefined)) {
    /// @resolution.name source=value target=keep.value
    /// @resolution.operator source="value !== (other as int32 | undefined)" type=boolean operator="!==" kind=builtin operands=[value as int32 | undefined families=(integer | undefined), other as int32 | undefined as int32 | undefined families=(integer | undefined)]
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=keep.value
    /// @resolution.name source=other target=keep.other
    /// @resolution.place source=other placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=other root=keep.other

        return value;
        /// @resolution.name source=value target=keep.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=keep.value

    }
    return 0;
}
"#,
        r#"
/// @diagnostic.warning id=redundant-cast message="cast to 'int32 | undefined' has no effect"
/// @diagnostic.label line=3 column=26 span="as" line_source="if (value !== (other as int32 | undefined)) {"
/// @diagnostic.suggestion message="remove the cast" applicability=automatic patched="if (value !== (other)) {"
/// @diagnostic.error id=return-not-assignable message="type 'int32 | undefined' is not assignable to the declared result type 'int32'"
/// @diagnostic.label line=4 column=16 span="value" line_source="return value;"
/// @diagnostic.note message="expected 'int32', found 'undefined'"
"#,
    );
}

/// Narrow a union by a function-type `is` guard on both branches.
#[test]
fn test_function_type_guard_narrows_both_branches() {
    let session = TestSession::single(
        r#"
function render(message: string | (() => string) | undefined): string {
    if (message is () => string) {
        return message();
    }
    return message ?? "fallback";
}
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
function render(message: string | (() => string) | undefined): string {
    if (message is () => string) {
        return message();
    }
    return message ?? "fallback";
}

=== dir ===
function render(message: string | (() => string) | undefined): string {
/// @type.symbol symbol=render type=(string | () => string | undefined) => string
/// @type.symbol symbol=render.message source="message: string | (() => string) | undefined" type=string | () => string | undefined

    if (message is () => string) {
    /// @resolution.name source=message target=render.message
    /// @resolution.guard source="message is () => string" kind=is value=string | () => string | undefined target=() => string predicate="string | () => string | undefined is type(() => string)" narrowed=Narrow<string | () => string | undefined, () => string>
    /// @resolution.place source=message placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=message root=render.message

        return message();
        /// @resolution.name source=message target=render.message
        /// @resolution.call source=message() parameters=() return=string kind=expression target=expression
        /// @resolution.place source=message placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=message root=render.message
        /// @resolution.narrowing source=message union=string | () => string | undefined arms=() => string

    }
    return message ?? "fallback";
    /// @resolution.name source=message target=render.message
    /// @resolution.operator source="message ?? \"fallback\"" type=string operator="??" kind=builtin operands=[message as string | undefined families=(string | undefined), "fallback" as "fallback" families=(string)]
    /// @resolution.place source=message placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=message root=render.message
    /// @resolution.narrowing source=message union=string | () => string | undefined arms=string | undefined

}
"#, r#"
"#);
}

/// Function type guards narrow arms stored behind a type alias.
#[test]
fn test_function_type_guard_narrows_through_an_alias() {
    let session = TestSession::single(
        r#"
type Message = string | (() => string);

function render(message: Message | undefined): string {
    if (message is () => string) {
        return message();
    }
    return message ?? "fallback";
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Message = string | (() => string);

function render(message: string | (() => string) | undefined): string {
    if (message is () => string) {
        return message();
    }
    return message ?? "fallback";
}

=== dir ===
type Message = string | (() => string);
/// @type.symbol symbol=Message source="type Message = string | (() => string)" type=string | () => string
/// @definition.type symbol=Message source="type Message = string | (() => string)" value=string | () => string

function render(message: Message | undefined): string {
/// @type.symbol symbol=render type=(string | () => string | undefined) => string
/// @type.symbol symbol=render.message source="message: Message | undefined" type=string | () => string | undefined
/// @resolution.name source=Message target=Message

    if (message is () => string) {
    /// @resolution.name source=message target=render.message
    /// @resolution.guard source="message is () => string" kind=is value=string | () => string | undefined target=() => string predicate="string | () => string | undefined is type(() => string)" narrowed=Narrow<string | () => string | undefined, () => string>
    /// @resolution.place source=message placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=message root=render.message

        return message();
        /// @resolution.name source=message target=render.message
        /// @resolution.call source=message() parameters=() return=string kind=expression target=expression
        /// @resolution.place source=message placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=message root=render.message
        /// @resolution.narrowing source=message union=string | () => string | undefined arms=() => string

    }
    return message ?? "fallback";
    /// @resolution.name source=message target=render.message
    /// @resolution.operator source="message ?? \"fallback\"" type=string operator="??" kind=builtin operands=[message as string | undefined families=(string | undefined), "fallback" as "fallback" families=(string)]
    /// @resolution.place source=message placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=message root=render.message
    /// @resolution.narrowing source=message union=string | () => string | undefined arms=string | undefined

}
"#,
        r#"
"#,
    );
}

/// Coalesce splits nullish arms stored behind a type alias.
#[test]
fn test_coalesce_splits_aliased_nullish_arms() {
    let session = TestSession::single(
        r#"
type Nothing = null | undefined;

function pick(value: string | Nothing): string {
    return value ?? "fallback";
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Nothing = null | undefined;

function pick(value: string | null | undefined): string {
    return value ?? "fallback";
}

=== dir ===
type Nothing = null | undefined;
/// @type.symbol symbol=Nothing source="type Nothing = null | undefined" type=null | undefined
/// @definition.type symbol=Nothing source="type Nothing = null | undefined" value=null | undefined

function pick(value: string | Nothing): string {
/// @type.symbol symbol=pick type=(string | null | undefined) => string
/// @type.symbol symbol=pick.value source="value: string | Nothing" type=string | null | undefined
/// @resolution.name source=Nothing target=Nothing

    return value ?? "fallback";
    /// @resolution.name source=value target=pick.value
    /// @resolution.operator source="value ?? \"fallback\"" type=string operator="??" kind=builtin operands=[value as string | null | undefined families=(string | null | undefined), "fallback" as "fallback" families=(string)]
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=pick.value

}
"#,
        r#"
"#,
    );
}

/// Compare characters with builtin relational operators.
#[test]
fn test_compare_characters_relationally() {
    let session = TestSession::single(
        r#"
function isLowercase(character: char): boolean {
    return character >= 'a' && character <= 'z';
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function isLowercase(character: char): boolean {
    return character >= 'a' && character <= 'z';
}

=== dir ===
function isLowercase(character: char): boolean {
/// @type.symbol symbol=isLowercase type=(char) => boolean
/// @type.symbol symbol=isLowercase.character source="character: char" type=char

    return character >= 'a' && character <= 'z';
    /// @resolution.name source=character target=isLowercase.character
    /// @resolution.operator source="character >= 'a' && character <= 'z'" type=boolean operator="&&" kind=builtin operands=[character >= 'a' as boolean families=(boolean), character <= 'z' as boolean families=(boolean)]
    /// @resolution.operator source="character >= 'a'" type=boolean operator=">=" kind=builtin operands=[character as char families=(character), 'a' as 'a' families=(character)]
    /// @resolution.place source=character placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=character root=isLowercase.character
    /// @resolution.name source=character target=isLowercase.character
    /// @resolution.operator source="character <= 'z'" type=boolean operator="<=" kind=builtin operands=[character as char families=(character), 'z' as 'z' families=(character)]
    /// @resolution.place source=character placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=character root=isLowercase.character

}
"#,
        r#"
"#,
    );
}

/// Accept an object spread that overrides an earlier property.
#[test]
fn test_accept_a_spread_that_overrides_an_earlier_property() {
    let session = TestSession::single(
        r#"
function describe(): { reason: string } {
    return {
        reason: "",
        ...{ reason: "generated" },
    };
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function describe(): { reason: string } {
    return {
        reason: "",
        ...{ reason: "generated" },
    };
}

=== dir ===
function describe(): { reason: string } {
/// @type.symbol symbol=describe type=() => { reason: string }
/// @type.symbol symbol=describe.reason source="reason: string" type=string

    return {
        reason: "",
        ...{ reason: "generated" },
    };
}
"#,
        r#"
"#,
    );
}

/// Match decorator object arguments with spread overrides against newtype backings.
#[test]
fn test_spread_decorator_argument_matches_newtype_backing() {
    let session = TestSession::single(
        r#"
@allow("constant-condition", {
    if: true,
    reason: "",
    ...{ reason: "generated" },
})
function decorated(): boolean {
    return true;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
@allow("constant-condition", {
    if: true,
    reason: "",
    ...{ reason: "generated" },
})
function decorated(): boolean {
    return true;
}

=== dir ===
@allow("constant-condition", {
/// @resolution.name source=allow target=allow

    if: true,
    reason: "",
    ...{ reason: "generated" },
})
function decorated(): boolean {
/// @type.symbol symbol=decorated type=() => boolean

    return true;
}
"#,
        r#"

"#,
    );
}

/// Look up prelude extension methods on primitive char receivers.
#[test]
fn test_call_character_extension_methods() {
    let session = TestSession::single(
        r#"
function classify(character: char): boolean {
    return character.isAsciiAlphabetic() || character.isAsciiDigit();
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function classify(character: char): boolean {
    return character.isAsciiAlphabetic<"frame">() || character.isAsciiDigit<"frame">();
}

=== dir ===
function classify(character: char): boolean {
/// @type.symbol symbol=classify type=(char) => boolean
/// @type.symbol symbol=classify.character source="character: char" type=char

    return character.isAsciiAlphabetic() || character.isAsciiDigit();
    /// @resolution.name source=character target=classify.character
    /// @resolution.member source=character.isAsciiAlphabetic receiver=char type=<Character.isAsciiAlphabetic.'a>(this: &Character.isAsciiAlphabetic.'a readonly char) => boolean kind=symbol target_receiver=char target=Character.isAsciiAlphabetic
    /// @resolution.call source=character.isAsciiAlphabetic() parameters=() return=boolean regions=("frame" & "local") kind=symbol target=Character.isAsciiAlphabetic receiver=char adjustments=(borrow(&'frame readonly char)) instance="Character.isAsciiAlphabetic<\"frame\" & \"local\">"
    /// @resolution.operator source="character.isAsciiAlphabetic() || character.isAsciiDigit()" type=boolean operator="||" kind=builtin operands=[character.isAsciiAlphabetic() as boolean families=(boolean), character.isAsciiDigit() as boolean families=(boolean)]
    /// @resolution.place source=character placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=character root=classify.character
    /// @generic.instantiation id="Character.isAsciiAlphabetic<\"frame\" & \"local\">" template=Character.isAsciiAlphabetic arguments=("frame" & "local")
    /// @resolution.name source=character target=classify.character
    /// @resolution.member source=character.isAsciiDigit receiver=char type=<Character.isAsciiDigit.'a>(this: &Character.isAsciiDigit.'a readonly char) => boolean kind=symbol target_receiver=char target=Character.isAsciiDigit
    /// @resolution.call source=character.isAsciiDigit() parameters=() return=boolean regions=("frame" & "local") kind=symbol target=Character.isAsciiDigit receiver=char adjustments=(borrow(&'frame readonly char)) instance="Character.isAsciiDigit<\"frame\" & \"local\">"
    /// @resolution.place source=character placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=character root=classify.character
    /// @generic.instantiation id="Character.isAsciiDigit<\"frame\" & \"local\">" template=Character.isAsciiDigit arguments=("frame" & "local")

}
"#,
        r#"
"#,
    );
}

/// Look up module extension methods on primitive receivers.
#[test]
fn test_call_local_primitive_extension_methods() {
    let session = TestSession::single(
        r#"
extension Doubling of int32 {
    doubled(this): int32 {
        this * 2
    }
}

function double(value: int32): int32 {
    return value.doubled();
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
extension Doubling of int32 {
    doubled(this): int32 {
        this * 2
    }
}

function double(value: int32): int32 {
    return value.doubled();
}

=== dir ===
extension Doubling of int32 {
/// @definition.extension symbol=Doubling form=local target=int32
/// @definition.method symbol=Doubling.doubled slot=doubled type=(this: int32) => int32

    doubled(this): int32 {
    /// @type.symbol symbol=Doubling.doubled type=(this: int32) => int32
    /// @type.symbol symbol=Doubling.doubled.this source=this type=int32

        this * 2
        /// @resolution.operator source="this * 2" type=int32 operator="*" kind=builtin operands=[this as int32 families=(integer), 2 as int32 families=(integer)]
        /// @resolution.receiver source=this kind=this declaration=Doubling type=int32
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this

    }
}

function double(value: int32): int32 {
/// @type.symbol symbol=double type=(int32) => int32
/// @type.symbol symbol=double.value source="value: int32" type=int32

    return value.doubled();
    /// @resolution.name source=value target=double.value
    /// @resolution.member source=value.doubled receiver=int32 type=(this: int32) => int32 kind=symbol target_receiver=int32 target=Doubling.doubled
    /// @resolution.call source=value.doubled() parameters=() return=int32 kind=symbol target=Doubling.doubled receiver=int32
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=double.value

}
"#,
        r#"
"#,
    );
}

/// Unwrap a result value with the try operator.
#[test]
fn test_unwrap_a_result_with_the_try_operator() {
    let session = TestSession::single(
        r#"
declare function parseCount(source: string): Result<int32, string>;

function incrementCount(source: string): Result<int32, string> {
    const count = parseCount(source)?;
    return Result.ok(count + 1);
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function parseCount(source: string): Result<int32, string>;

function incrementCount(source: string): Result<int32, string> {
    const count: int32 = parseCount(source)?;
    return Result.ok<int32, string>(count + 1);
}

=== dir ===
declare function parseCount(source: string): Result<int32, string>;
/// @type.symbol symbol=parseCount source="declare function parseCount(source: string): Result<int32, string>" type=(string) => Result<int32, string>
/// @resolution.name source=Result target=Result

function incrementCount(source: string): Result<int32, string> {
/// @type.symbol symbol=incrementCount type=(string) => Result<int32, string>
/// @type.symbol symbol=incrementCount.source source="source: string" type=string
/// @resolution.name source=Result target=Result

    const count = parseCount(source)?;
    /// @type.symbol symbol=incrementCount.count source=count type=int32
    /// @resolution.pattern source=count kind=binding target=incrementCount.count
    /// @resolution.name source=parseCount target=parseCount
    /// @resolution.call source=parseCount(source) parameters=(string) arguments=(provided(source) as string) return=Result<int32, string> kind=symbol target=parseCount
    /// @resolution.residual source=parseCount(source)? target=callable residual=TryResidual<Result<int32, string>> branch="branch(parameters=(), arguments=(), return=ControlFlow<Result<never, string>, int32>)" from_residual="fromResidual(parameters=(Result<never, string>), arguments=(supplied(0) as Result<never, string>), return=Result<int32, string>)"
    /// @generic.instantiation id="branch<int32, string>" template=branch arguments=(int32, string)
    /// @generic.instantiation id="fromResidual<int32, string, string>" template=fromResidual arguments=(int32, string, string)
    /// @resolution.name source=source target=incrementCount.source
    /// @resolution.place source=source placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=source root=incrementCount.source

    return Result.ok(count + 1);
    /// @resolution.name source=Result target=Result
    /// @resolution.member source=Result.ok receiver=Result type=(T#1) => Result<T#1, E#1> kind=symbol target_receiver=Result target=ok#1
    /// @resolution.call source="Result.ok(count + 1)" parameters=(int32) arguments=(provided(count + 1) as int32) return=Result<int32, string> kind=symbol target=ok#1 instance="Result<int32, string>.<extension#1>.ok#1"
    /// @generic.instantiation id="ok#1<int32, string>" template=ok#1 arguments=(int32, string)
    /// @resolution.name source=count target=incrementCount.count
    /// @resolution.operator source="count + 1" type=int32 operator="+" kind=builtin operands=[count as int32 families=(integer), 1 as int32 families=(integer)]
    /// @resolution.place source=count placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=count root=incrementCount.count

}
"#,
        r#"
"#,
    );
}

/// Match a result exhaustively over its ok and error arms.
#[test]
fn test_match_a_result_exhaustively() {
    let session = TestSession::single(
        r#"
function unwrap(result: Result<int32, string>): int32 {
    const value = match (result) {
        Err { error: _ } => 0
        Ok { value } => value
    };
    return value;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function unwrap(result: Result<int32, string>): int32 {
    const value: int32 = match (result) {
        Err { error: _ } => 0
        Ok { value } => value
    };
    return value;
}

=== dir ===
function unwrap(result: Result<int32, string>): int32 {
/// @type.symbol symbol=unwrap type=(Result<int32, string>) => int32
/// @type.symbol symbol=unwrap.result source="result: Result<int32, string>" type=Result<int32, string>
/// @resolution.name source=Result target=Result

    const value = match (result) {
    /// @type.symbol symbol=unwrap.value#2 source=value type=int32
    /// @resolution.pattern source=value kind=binding target=unwrap.value#2
    /// @resolution.coverage exhaustive=true disjoint=true
    /// @resolution.name source=result target=unwrap.result
    /// @resolution.place source=result placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=result root=unwrap.result

        Err { error: _ } => 0
        /// @resolution.name source=Err target=Err
        /// @resolution.pattern source="Err { error: _ }" kind=nominal_object adjustments=(newtype.payload(Result, Ok<int32> | Err<string>), union.payload(Ok<int32> | Err<string>, Err<string>, Err<string>)) target=Err instance=Err<string> fields={ Err.error: _ }
        /// @generic.instantiation id="Result<int32, string>" template=Result arguments=(int32, string)
        /// @generic.instantiation id=Err<string> template=Err arguments=(string)
        /// @resolution.pattern source=_ kind=wildcard

        Ok { value } => value
        /// @resolution.name source=Ok target=Ok
        /// @resolution.pattern source="Ok { value }" kind=nominal_object adjustments=(newtype.payload(Result, Ok<int32> | Err<string>), union.payload(Ok<int32> | Err<string>, Ok<int32>, Ok<int32>)) target=Ok instance=Ok<int32> fields={ Ok.value }
        /// @generic.instantiation id=Ok<int32> template=Ok arguments=(int32)
        /// @type.symbol symbol=unwrap.value#1 source=value type=int32
        /// @resolution.name source=value target=unwrap.value#1
        /// @resolution.place source=value placement="local" lifetime="frame" access="immutable"
        /// @resolution.access source=value root=unwrap.value#1

    };
    return value;
    /// @resolution.name source=value target=unwrap.value#2
    /// @resolution.place source=value placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=value root=unwrap.value#2

}
"#,
        r#"
"#,
    );
}

/// Retain the access each borrow requires from the place it lends.
#[test]
fn test_retain_required_borrow_access() {
    let session = TestSession::single(
        r#"
declare function fill(buffer: &[int32]): void;

function prepare(values: [int32]): void {
    fill(&values);
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_flows(),
        r#"
=== annotated ===
declare function fill<'a>(buffer: &[int32]): void;

function prepare(values: [int32]): void {
    fill<"managed">(&values);
}

=== dir ===
declare function fill(buffer: &[int32]): void;
/// @generic.template symbol=fill parameters=('a)
/// @type.symbol symbol=fill source="declare function fill(buffer: &[int32]): void" type=<fill.'a>(&fill.'a Slice<int32>) => void
/// @flow.use symbol=fill uses=read

function prepare(values: [int32]): void {
/// @type.symbol symbol=prepare type=(Slice<int32>) => void
/// @type.symbol symbol=prepare.values source="values: [int32]" type=Slice<int32>
/// @flow.use symbol=values uses=read+mutable

    fill(&values);
    /// @resolution.name source=fill target=fill
    /// @resolution.call source=fill(&values) parameters=(&'managed Slice<int32>) arguments=(provided(&values) as &'managed Slice<int32>) return=void regions=("managed" & "local") kind=symbol target=fill instance="fill<\"managed\" & \"local\">"
    /// @generic.instantiation id="fill<\"managed\" & \"local\">" template=fill arguments=("managed" & "local")
    /// @resolution.name source=values target=prepare.values
    /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values root=prepare.values
    /// @flow.access source=values root=prepare.values uses=read+mutable

}
"#,
        r#"
"#,
    );
}

/// Type spread arguments and spread literals contextually against iterable expectations.
#[test]
fn test_spread_arguments_type_contextually() {
    let session = TestSession::single(
        r#"
declare function consume(values: Iterable<int32>): void;

function feed(output: Array<int32>, values: [int32]): void {
    output.push(1, ...[2, 3], 4);
    consume([...values]);
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function consume(values: Iterable<int32>): void;

function feed(output: int32[], values: [int32]): void {
    output.push<int32, "managed">(1, ...[2, 3], 4);
    consume([...values] as Iterable<int32>);
}

=== dir ===
declare function consume(values: Iterable<int32>): void;
/// @type.symbol symbol=consume source="declare function consume(values: Iterable<int32>): void" type=(Iterable<int32>) => void
/// @resolution.name source=Iterable target=Iterable

function feed(output: Array<int32>, values: [int32]): void {
/// @type.symbol symbol=feed type=(int32[], Slice<int32>) => void
/// @type.symbol symbol=feed.output source="output: Array<int32>" type=int32[]
/// @resolution.name source=Array target=Array
/// @type.symbol symbol=feed.values source="values: [int32]" type=Slice<int32>

    output.push(1, ...[2, 3], 4);
    /// @resolution.name source=output target=feed.output
    /// @resolution.member source=output.push receiver=int32[] type=<push.'a>(this: &push.'a int32[], ...int32[]) => isize kind=symbol target_receiver=int32[] target=push
    /// @resolution.call source="output.push(1, ...[2, 3], 4)" parameters=(int32[]) arguments=(rest(provided(1) as int32, spread(provided(...[2, 3]) as ^int32[], iterator=iterator#1(parameters=(), arguments=(), return=Iterator<int32>), next=dynamic(Iterator<int32> as Iterator<int32>, Iterator.next)(parameters=(), arguments=(), return=IteratorResult<int32, void>, regions=("managed" & "local"))) as int32, provided(4) as int32) pack=arrayFromOwnedSlice as int32) return=isize regions=("managed" & "local") kind=symbol target=push receiver=int32[] adjustments=(borrow(&'managed int32[])) instance="Array<int32>.<extension#6>.push<\"managed\" & \"local\">"
    /// @resolution.place source=output placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=output root=feed.output
    /// @generic.instantiation id="push<int32, \"managed\" & \"local\">" template=push arguments=(int32, "managed" & "local")
    /// @generic.instantiation id=iterator#1<int32> template=iterator#1 arguments=(int32)
    /// @generic.instantiation id=push<int32> template=push arguments=(int32)
    /// @resolution.call source=[2, 3] parameters=(^Slice<int32>) arguments=(rest(provided(2) as int32, provided(3) as int32) as int32) return=int32[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int32>
    /// @generic.instantiation id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)

    consume([...values]);
    /// @resolution.name source=consume target=consume
    /// @resolution.call source=consume([...values]) parameters=(Iterable<int32>) arguments=(provided([...values]) as Iterable<int32>) return=void kind=symbol target=consume
    /// @resolution.call source=[...values] parameters=(^Slice<int32>) arguments=(rest(spread(provided(...values) as Slice<int32>, iterator=iterator#1(parameters=(), arguments=(), return=Iterator<int32>, regions=("managed" & "local")), next=dynamic(Iterator<int32> as Iterator<int32>, Iterator.next)(parameters=(), arguments=(), return=IteratorResult<int32, void>, regions=("managed" & "local"))) as int32) as int32) return=int32[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int32>
    /// @generic.instantiation id="iterator#1<int32, \"managed\" & \"local\">" template=iterator#1 arguments=(int32, "managed" & "local")
    /// @resolution.name source=values target=feed.values
    /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values root=feed.values

}
"#,
        r#"
"#,
    );
}
