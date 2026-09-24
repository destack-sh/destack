use crate::tests::{DirRows, TestSession};

#[test]
fn test_call_selects_first_applicable_overload_in_declaration_order() {
    let session = TestSession::single(
        r#"
function parse(value: string): "string" {
    return "string";
}

function parse(value: "id"): "literal" {
    return "literal";
}

const result = parse("id");
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked()
            .with_node_types()
            .without_reference_types(),
        r#"
=== annotated ===
function parse(value: string): "string" {
    return "string";
}

function parse(value: "id"): "literal" {
    return "literal";
}

const result: "string" = parse("id");

=== dir ===
function parse(value: string): "string" {
/// @type.symbol symbol=parse#1 type=(string) => "string"
/// @type.symbol symbol=parse.value#1 source="value: string" type=string

    return "string";
    /// @type.node source="\"string\"" type="string"

}

function parse(value: "id"): "literal" {
/// @type.symbol symbol=parse#2 type=("id") => "literal"
/// @type.symbol symbol=parse.value#2 source="value: \"id\"" type="id"

    return "literal";
    /// @type.node source="\"literal\"" type="literal"

}

const result = parse("id");
/// @type.symbol symbol=result source=result type="string"
/// @resolution.pattern source=result kind=binding target=result
/// @type.node source="parse(\"id\")" type="string"
/// @resolution.name source=parse target=[parse#1, parse#2]
/// @resolution.call source="parse(\"id\")" parameters=(string) arguments=(provided("id") as string) return="string" kind=symbol target=parse#1
/// @type.node source="\"id\"" type="id"
"#,
    );
}

#[test]
fn test_call_selects_first_applicable_overload_ignoring_expected_return() {
    let session = TestSession::single(
        r#"
function choose(value: string): string {
    return "text";
}

function choose(value: string): int32 {
    return 0;
}

const result: int32 = choose("x");
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked()
            .with_node_types()
            .without_reference_types(),
        r#"
=== annotated ===
function choose(value: string): string {
    return "text";
}

function choose(value: string): int32 {
    return 0;
}

const result: int32 = choose("x");

=== dir ===
function choose(value: string): string {
/// @type.symbol symbol=choose#1 type=(string) => string
/// @type.symbol symbol=choose.value#1 source="value: string" type=string

    return "text";
    /// @type.node source="\"text\"" type="text"

}

function choose(value: string): int32 {
/// @type.symbol symbol=choose#2 type=(string) => int32
/// @type.symbol symbol=choose.value#2 source="value: string" type=string

    return 0;
    /// @type.node source=0 type=0

}

const result: int32 = choose("x");
/// @type.symbol symbol=result source=result type=int32
/// @resolution.pattern source=result kind=binding target=result
/// @type.node source="choose(\"x\")" type=string
/// @resolution.name source=choose target=[choose#1, choose#2]
/// @resolution.call source="choose(\"x\")" parameters=(string) arguments=(provided("x") as string) return=string kind=symbol target=choose#1
/// @type.node source="\"x\"" type="x"
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'string' is not assignable to type 'int32'"
/// @diagnostic.label line=10 column=23 span="choose(\"x\")" line_source="const result: int32 = choose(\"x\");"
/// @diagnostic.related line=10 column=15 span="int32" line_source="const result: int32 = choose(\"x\");" message="expected due to this annotation"
"#,
    );
}

#[test]
fn test_call_with_no_matching_overload_reports_error() {
    let session = TestSession::single(
        r#"
declare function parse(value: string): int32;
declare function parse(value: int32): int32;

parse(true);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked()
            .with_node_types()
            .without_reference_types(),
        r#"
=== annotated ===
declare function parse(value: string): int32;
declare function parse(value: int32): int32;

parse(true);

=== dir ===
declare function parse(value: string): int32;
/// @type.symbol symbol=parse#1 source="declare function parse(value: string): int32" type=(string) => int32

declare function parse(value: int32): int32;
/// @type.symbol symbol=parse#2 source="declare function parse(value: int32): int32" type=(int32) => int32

parse(true);
/// @type.node source=parse(true) type=<error>
/// @resolution.name source=parse target=[parse#1, parse#2]
/// @resolution.rejected source=parse(true)
/// @type.node source=true type=true
"#,
        r#"
/// @diagnostic.error id=no-matching-call message="no overload matches arguments ('true')"
/// @diagnostic.label line=5 column=1 span="parse(true)" line_source="parse(true);"
/// @diagnostic.note message="the candidate '(value: string) => int32' rejects argument 0: 'true' is not assignable to 'string'"
/// @diagnostic.note message="the candidate '(value: int32) => int32' rejects argument 0: 'true' is not assignable to 'int32'"
"#,
    );
}

#[test]
fn test_concrete_function_declaration_requires_body() {
    let session = TestSession::single(
        r#"
function parse(value: string): int32;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().without_reference_types(),
        r#"
=== annotated ===
function parse(value: string): int32;

=== dir ===
function parse(value: string): int32;
/// @type.symbol symbol=parse source="function parse(value: string): int32" type=(string) => int32
"#,
        r#"
/// @diagnostic.error id=missing-declaration-body message="declaration 'parse' requires a body"
/// @diagnostic.label line=2 column=10 span="parse" line_source="function parse(value: string): int32;"
"#,
    );
}

#[test]
fn test_require_bodies_on_extension_members_despite_decorators() {
    let session = TestSession::single(
        r#"
struct Buffer {
    length: usize;
}

extension of Buffer {
    @intrinsic("buffer.capacity")
    get capacity(): usize;

    trailing(): usize;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Buffer {
    length: usize;
}

extension of Buffer {
    @intrinsic("buffer.capacity")
    get capacity(): usize;

    trailing(): usize;
}

=== dir ===
struct Buffer {
/// @type.symbol symbol=Buffer type=Buffer
/// @definition.struct symbol=Buffer
/// @definition.field symbol=Buffer.length source="length: usize" key=length type=usize

    length: usize;
    /// @type.symbol symbol=Buffer.length source="length: usize" type=usize

}

extension of Buffer {
/// @definition.extension symbol=<module>#2 form=local target=Buffer
/// @definition.method symbol=capacity source="get capacity(): usize" slot=capacity role=getter type=<capacity.'a>(this: &capacity.'a readonly Buffer) => usize
/// @definition.method symbol=trailing source="trailing(): usize" slot=trailing type=<trailing.'a>(this: &trailing.'a readonly Buffer) => usize
/// @resolution.name source=Buffer target=Buffer

    @intrinsic("buffer.capacity")
    /// @resolution.name source=intrinsic target=intrinsic

    get capacity(): usize;
    /// @generic.template symbol=capacity parameters=('a)
    /// @type.symbol symbol=capacity source="get capacity(): usize" type=<capacity.'a>(this: &capacity.'a readonly Buffer) => usize

    trailing(): usize;
    /// @generic.template symbol=trailing parameters=('a)
    /// @type.symbol symbol=trailing source="trailing(): usize" type=<trailing.'a>(this: &trailing.'a readonly Buffer) => usize

}
"#,
        r#"
/// @diagnostic.error id=missing-declaration-body message="declaration 'get' requires a body"
/// @diagnostic.label line=8 column=9 span="capacity" line_source="get capacity(): usize;"
/// @diagnostic.error id=missing-declaration-body message="declaration 'trailing' requires a body"
/// @diagnostic.label line=10 column=5 span="trailing" line_source="trailing(): usize;"
"#,
    );
}

#[test]
fn test_keep_argument_bindings_when_a_call_is_rejected() {
    let session = TestSession::single(
        r#"
function greet(name: string, count: int32): string {
    return name;
}

const value = greet("hi");
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function greet(name: string, count: int32): string {
    return name;
}

const value = greet("hi");

=== dir ===
function greet(name: string, count: int32): string {
/// @type.symbol symbol=greet type=(string, int32) => string
/// @type.symbol symbol=greet.name source="name: string" type=string
/// @type.symbol symbol=greet.count source="count: int32" type=int32

    return name;
    /// @resolution.name source=name target=greet.name
    /// @resolution.place source=name placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=name root=greet.name

}

const value = greet("hi");
/// @type.symbol symbol=value source=value type=<error>
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=greet target=greet
/// @resolution.call source="greet(\"hi\")" parameters=(string, int32) arguments=(provided("hi") as string, omitted as int32) return=string kind=symbol target=greet
"#,
        r#"
/// @diagnostic.error id=wrong-argument-count message="expected 2 arguments, but got 1 argument(s)"
/// @diagnostic.label line=6 column=15 span="greet(\"hi\")" line_source="const value = greet(\"hi\");"
"#,
    );
}

/// Reject a bare reference to an overload group without a selecting call.
#[test]
fn test_bare_overload_reference_reports_ambiguous_overload() {
    let session = TestSession::single(
        r#"
function parse(value: int32): int32 {
    value
}

function parse(value: string): string {
    value
}

const parser = parse;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function parse(value: int32): int32 {
    value
}

function parse(value: string): string {
    value
}

const parser = parse;

=== dir ===
function parse(value: int32): int32 {
/// @type.symbol symbol=parse#1 type=(int32) => int32
/// @type.symbol symbol=parse.value#1 source="value: int32" type=int32

    value
    /// @resolution.name source=value target=parse.value#1
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=parse.value#1

}

function parse(value: string): string {
/// @type.symbol symbol=parse#2 type=(string) => string
/// @type.symbol symbol=parse.value#2 source="value: string" type=string

    value
    /// @resolution.name source=value target=parse.value#2
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=parse.value#2

}

const parser = parse;
/// @type.symbol symbol=parser source=parser type=<error>
/// @resolution.pattern source=parser kind=binding target=parser
/// @resolution.name source=parse target=[parse#1, parse#2]
"#,
        r#"
/// @diagnostic.error id=ambiguous-overload message="overload 'parse' is ambiguous without a call"
/// @diagnostic.label line=10 column=16 span="parse" line_source="const parser = parse;"
/// @diagnostic.related line=2 column=10 span="parse" line_source="function parse(value: int32): int32 {" message="one candidate is declared here"
/// @diagnostic.related line=6 column=10 span="parse" line_source="function parse(value: string): string {" message="one candidate is declared here"
"#,
    );
}

#[test]
fn test_annotated_reference_selects_first_conforming_overload() {
    let session = TestSession::single(
        r#"
function render(value: int32): int32 {
    return value;
}

function render(value: string): string {
    return value;
}

const text: (value: string) => string = render;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function render(value: int32): int32 {
    return value;
}

function render(value: string): string {
    return value;
}

const text: (value: string) => string = render;

=== dir ===
function render(value: int32): int32 {
/// @type.symbol symbol=render#1 type=(int32) => int32
/// @type.symbol symbol=render.value#1 source="value: int32" type=int32

    return value;
    /// @resolution.name source=value target=render.value#1
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=render.value#1

}

function render(value: string): string {
/// @type.symbol symbol=render#2 type=(string) => string
/// @type.symbol symbol=render.value#2 source="value: string" type=string

    return value;
    /// @resolution.name source=value target=render.value#2
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=render.value#2

}

const text: (value: string) => string = render;
/// @type.symbol symbol=text source=text type=(string) => string
/// @resolution.pattern source=text kind=binding target=text
/// @type.symbol symbol=value source="value: string" type=string
/// @resolution.name source=render target=[render#1, render#2]
/// @resolution.function source=render type=Function<(string,), string, "readonly"> target=render#2
"#,
    );
}

#[test]
fn test_reference_without_conforming_overload_reports_plural_group() {
    let session = TestSession::single(
        r#"
function render(value: int32): int32 {
    return value;
}

function render(value: string): string {
    return value;
}

const chosen: (value: boolean) => boolean = render;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function render(value: int32): int32 {
    return value;
}

function render(value: string): string {
    return value;
}

const chosen: (value: boolean) => boolean = render;

=== dir ===
function render(value: int32): int32 {
/// @type.symbol symbol=render#1 type=(int32) => int32
/// @type.symbol symbol=render.value#1 source="value: int32" type=int32

    return value;
    /// @resolution.name source=value target=render.value#1
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=render.value#1

}

function render(value: string): string {
/// @type.symbol symbol=render#2 type=(string) => string
/// @type.symbol symbol=render.value#2 source="value: string" type=string

    return value;
    /// @resolution.name source=value target=render.value#2
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=render.value#2

}

const chosen: (value: boolean) => boolean = render;
/// @type.symbol symbol=chosen source=chosen type=(boolean) => boolean
/// @resolution.pattern source=chosen kind=binding target=chosen
/// @type.symbol symbol=value source="value: boolean" type=boolean
/// @resolution.name source=render target=[render#1, render#2]
"#,
        r#"
/// @diagnostic.error id=ambiguous-overload message="overload 'render' is ambiguous without a call"
/// @diagnostic.label line=10 column=45 span="render" line_source="const chosen: (value: boolean) => boolean = render;"
/// @diagnostic.related line=2 column=10 span="render" line_source="function render(value: int32): int32 {" message="one candidate is declared here"
/// @diagnostic.related line=6 column=10 span="render" line_source="function render(value: string): string {" message="one candidate is declared here"
"#,
    );
}

#[test]
fn test_select_generic_overload_with_block_closure_and_try() {
    let session = TestSession::single(
        r#"
import { Iterator } from "destack:iter";
import { Result } from "destack:error";

function sum(values: Iterator<int32>): Result<int32, string> {
    return values.reduce((result, value, index) => {
        const total = result?;

        Result.ok(total + value + index.truncate<int32>())
    }, Result.ok(0));
}
"#,
    );

    session.assert_dir("main.ds", DirRows::checked().with_node_types(), r#"
=== annotated ===
import { Result } from "destack:error";
import { Iterator } from "destack:iter";

function sum(values: Iterator<int32>): Result<int32, string> {
    return values.reduce<int32, Result<int32, string>>(
        (result: Result<int32, string>, value: int32, index: isize): Result<int32, string> => {
            const total: int32 = result?;

            Result.ok<int32, string>(total + value + index.truncate<int32>())
        },
        Result.ok<int32, string>(0),
    );
}

=== dir ===
import { Iterator } from "destack:iter";
import { Result } from "destack:error";

function sum(values: Iterator<int32>): Result<int32, string> {
/// @type.symbol symbol=sum type=(Iterator<int32>) => Result<int32, string>
/// @generic.instance id="Result<int32, string>" template=Result arguments=(int32, string)
/// @generic.instance id=Err<string> template=Err arguments=(string)
/// @generic.instance id=Iterator<int32> template=Iterator arguments=(int32)
/// @generic.instance id=Ok<int32> template=Ok arguments=(int32)
/// @type.symbol symbol=sum.values source="values: Iterator<int32>" type=Iterator<int32>
/// @resolution.name source=Iterator target=Iterator
/// @resolution.name source=Result target=Result

    return values.reduce((result, value, index) => {
    /// @type.node source=values.reduce type=(this: Iterator<int32>, (int32, int32, isize) => int32) => int32 & <Iterator.reduce.U>(this: Iterator<int32>, (Iterator.reduce.U, int32, isize) => Iterator.reduce.U, Iterator.reduce.U) => Iterator.reduce.U
    /// @type.node type=Result<int32, string>
    /// @resolution.name source=values target=sum.values
    /// @resolution.member source=values.reduce receiver=Iterator<int32> type=(this: Iterator<int32>, (int32, int32, isize) => int32) => int32 & <Iterator.reduce.U>(this: Iterator<int32>, (Iterator.reduce.U, int32, isize) => Iterator.reduce.U, Iterator.reduce.U) => Iterator.reduce.U kind=overload-set targets=[Iterator.reduce#1, Iterator.reduce#2]
    /// @resolution.call parameters=((Result<int32, string>, int32, isize) => Result<int32, string>, Result<int32, string>) arguments=(provided(argument) as (Result<int32, string>, int32, isize) => Result<int32, string>, provided(Result.ok(0)) as Result<int32, string>) return=Result<int32, string> kind=dynamic target=Iterator.reduce#2 receiver=Iterator<int32> constraint=Iterator<int32> generic_arguments=(int32, Result<int32, string>)
    /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values root=sum.values
    /// @generic.instantiation id=Iterator.reduce#1<int32> template=Iterator.reduce#1 arguments=(int32)
    /// @generic.instantiation id=Iterator.reduce#2<int32> template=Iterator.reduce#2 arguments=(int32)
    /// @type.symbol symbol=sum.symbol5 type=Function<(Result<int32, string>, int32, isize), Result<int32, string>, "readonly">
    /// @type.node type=Function<(Result<int32, string>, int32, isize), Result<int32, string>, "readonly">
    /// @type.symbol symbol=sum.symbol5.result source=result type=Result<int32, string>
    /// @type.symbol symbol=sum.symbol5.value source=value type=int32
    /// @type.symbol symbol=sum.symbol5.index source=index type=isize

        const total = result?;
        /// @type.symbol symbol=sum.symbol5.total source=total type=int32
        /// @resolution.pattern source=total kind=binding target=sum.symbol5.total
        /// @type.node source=result? type=int32
        /// @resolution.name source=result target=sum.symbol5.result
        /// @resolution.place source=result placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=result root=sum.symbol5.result
        /// @resolution.residual source=result? target=callable residual=TryResidual<Result<int32, string>> branch="branch(parameters=(), arguments=(), return=ControlFlow<Result<never, string>, int32>)" from_residual="fromResidual(parameters=(Result<never, string>), arguments=(supplied(0) as Result<never, string>), return=Result<int32, string>)"
        /// @generic.instantiation id="branch<int32, string>" template=branch arguments=(int32, string)
        /// @generic.instantiation id="fromResidual<int32, string, string>" template=fromResidual arguments=(int32, string, string)
        /// @generic.instance id="Break<Result<never, string>>" template=Break arguments=(Result<never, string>)
        /// @generic.instance id="ControlFlow<Result<never, string>, int32>" template=ControlFlow arguments=(Result<never, string>, int32)
        /// @generic.instance id="Result<never, string>" template=Result arguments=(never, string)
        /// @generic.instance id="branch<int32, string>" template=branch arguments=(int32, string)
        /// @generic.instance id="break<Result<never, string>, int32>" template=break arguments=(Result<never, string>, int32)
        /// @generic.instance id="continue<Result<never, string>, int32>" template=continue arguments=(Result<never, string>, int32)
        /// @generic.instance id="err#1<int32, string>" template=err#1 arguments=(int32, string)
        /// @generic.instance id="err#1<never, string>" template=err#1 arguments=(never, string)
        /// @generic.instance id="fromResidual<int32, string, string>" template=fromResidual arguments=(int32, string, string)
        /// @generic.instance id=Continue<int32> template=Continue arguments=(int32)
        /// @generic.instance id=Ok<never> template=Ok arguments=(never)
        /// @generic.instance id=from<string> template=from arguments=(string)

        Result.ok(total + value + index.truncate<int32>())
        /// @type.node source="Result.ok(total + value + index.truncate<int32>())" type=Result<int32, string>
        /// @type.node source=Result.ok type=(T#1) => Result<T#1, E#1>
        /// @resolution.name source=Result target=Result
        /// @resolution.member source=Result.ok receiver=Result type=(T#1) => Result<T#1, E#1> kind=symbol target_receiver=Result target=ok#1
        /// @resolution.call source="Result.ok(total + value + index.truncate<int32>())" parameters=(int32) arguments=(provided(total + value + index.truncate<int32>()) as int32) return=Result<int32, string> kind=symbol target=ok#1 instance="Result<int32, string>.<extension#1>.ok#1"
        /// @type.node source="total + value + index.truncate<int32>()" type=int32
        /// @type.node source="total + value" type=int32
        /// @resolution.name source=total target=sum.symbol5.total
        /// @resolution.operator source="total + value + index.truncate<int32>()" type=int32 operator="+" kind=builtin operands=[total + value as int32 families=(integer), index.truncate<int32>() as int32 families=(integer)]
        /// @resolution.operator source="total + value" type=int32 operator="+" kind=builtin operands=[total as int32 families=(integer), value as int32 families=(integer)]
        /// @resolution.place source=total placement="local" lifetime="frame" access="immutable"
        /// @resolution.access source=total root=sum.symbol5.total
        /// @resolution.name source=value target=sum.symbol5.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=sum.symbol5.value
        /// @type.node source=index.truncate type=<Cast.truncate.U: Integer>(this: isize) => Cast.truncate.U
        /// @type.node source=index.truncate<int32>() type=int32
        /// @resolution.name source=index target=sum.symbol5.index
        /// @resolution.member source=index.truncate receiver=isize type=<Cast.truncate.U: Integer>(this: isize) => Cast.truncate.U kind=symbol target_receiver=isize target=Cast.truncate
        /// @resolution.call source=index.truncate<int32>() parameters=() return=int32 kind=symbol target=Cast.truncate receiver=isize instance=Cast<isize>.truncate<int32>
        /// @resolution.place source=index placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=index root=sum.symbol5.index
        /// @generic.instantiation id="Cast.truncate<isize, int32>" template=Cast.truncate arguments=(isize, int32)
        /// @generic.instantiation id=Cast.truncate<isize> template=Cast.truncate arguments=(isize)
        /// @generic.instance id="Cast.truncate<isize, int32>" template=Cast.truncate arguments=(isize, int32)
        /// @generic.instance id="truncateInt<isize, int32>" template=truncateInt arguments=(isize, int32)

    }, Result.ok(0));
    /// @type.node source=Result.ok type=(T#1) => Result<T#1, E#1>
    /// @type.node source=Result.ok(0) type=Result<int32, string>
    /// @resolution.name source=Result target=Result
    /// @resolution.member source=Result.ok receiver=Result type=(T#1) => Result<T#1, E#1> kind=symbol target_receiver=Result target=ok#1
    /// @resolution.call source=Result.ok(0) parameters=(int32) arguments=(provided(0) as int32) return=Result<int32, string> kind=symbol target=ok#1 instance="Result<int32, string>.<extension#1>.ok#1"
    /// @generic.instantiation id="ok#1<int32, string>" template=ok#1 arguments=(int32, string)
    /// @generic.instance id="ok#1<int32, string>" template=ok#1 arguments=(int32, string)
    /// @type.node source=0 type=0

}
"#);
}

#[test]
fn test_widen_the_literal_initial_argument_of_a_reduce_overload() {
    let session = TestSession::single(
        r#"
newtype interface It<T, R = void> {
    next(this): R {
        todo("next")
    }

    reduce(this, reduce: (accumulator: T, value: T) => T): T {
        todo("reduce")
    }

    reduce<U>(this, reduce: (accumulator: U, value: T) => U, initial: U): U {
        todo("reduce")
    }
}

struct Wrap<T> {
    value: T;
}

function wrap<T>(value: T): Wrap<T> {
    Wrap { value }
}

function sum(values: It<int32>): Wrap<int32> {
    return values.reduce((result, value) => {
        wrap(value)
    }, wrap(0));
}
"#,
    );

    session.assert_dir("main.ds", DirRows::checked().with_node_types(), r#"
=== annotated ===
newtype interface It<in out T, out R = void> {
    next(this): R {
        todo("next" as string | undefined)
    }

    reduce(this, reduce: (accumulator: T, value: T) => T): T {
        todo("reduce" as string | undefined)
    }

    reduce<U>(this, reduce: (accumulator: U, value: T) => U, initial: U): U {
        todo("reduce" as string | undefined)
    }
}

struct Wrap<out T> {
    value: T;
}

function wrap<T>(value: T): Wrap<T> {
    Wrap<T> { value }
}

function sum(values: It<int32>): Wrap<int32> {
    return values.reduce<int32, void, Wrap<int32>>(
        (result: Wrap<int32>, value: int32): Wrap<int32> => {
            wrap<int32>(value)
        },
        wrap<int32>(0),
    );
}

=== dir ===
newtype interface It<T, R = void> {
/// @generic.template symbol=It parameters=(in out T#1, out R = void, this: It<T#1, R>)
/// @type.symbol symbol=It type=It
/// @definition.interface symbol=It template=(in out T#1, out R = void, this: It<T#1, R>) nominal=true
/// @definition.where symbol=It relation=satisfies left=this right=It<T#1, R>
/// @definition.method symbol=It.next slot=next type=(this: this) => R
/// @definition.method symbol=It.reduce#1 slot=reduce type=(this: this, (T#1, T#1) => T#1) => T#1
/// @definition.method symbol=It.reduce#2 slot=reduce type=<U>(this: this, (U, T#1) => U, U) => U
/// @type.symbol symbol=It.T source=T type=T#1
/// @type.symbol symbol=It.R source="R = void" type=R

    next(this): R {
    /// @type.symbol symbol=It.next type=(this: this) => R
    /// @type.symbol symbol=It.next.this source=this type=this
    /// @resolution.name source=R target=It.R

        todo("next")
        /// @type.node source="todo(\"next\")" type=never
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"next\")" parameters=(string | undefined) arguments=(provided("next") as string | undefined) return=never kind=symbol target=todo
        /// @type.node source="\"next\"" type="next"

    }

    reduce(this, reduce: (accumulator: T, value: T) => T): T {
    /// @type.symbol symbol=It.reduce#1 type=(this: this, (T#1, T#1) => T#1) => T#1
    /// @type.symbol symbol=It.reduce.this#1 source=this type=this
    /// @type.symbol symbol=It.reduce.reduce#1 source="reduce: (accumulator: T, value: T) => T" type=(T#1, T#1) => T#1
    /// @type.symbol symbol=It.reduce.accumulator#1 source="accumulator: T" type=T#1
    /// @resolution.name source=T target=It.T
    /// @type.symbol symbol=It.reduce.value#1 source="value: T" type=T#1
    /// @resolution.name source=T target=It.T
    /// @resolution.name source=T target=It.T
    /// @resolution.name source=T target=It.T

        todo("reduce")
        /// @type.node source="todo(\"reduce\")" type=never
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"reduce\")" parameters=(string | undefined) arguments=(provided("reduce") as string | undefined) return=never kind=symbol target=todo
        /// @type.node source="\"reduce\"" type="reduce"

    }

    reduce<U>(this, reduce: (accumulator: U, value: T) => U, initial: U): U {
    /// @generic.template symbol=It.reduce#2 parent=template#0 parameters=(U)
    /// @type.symbol symbol=It.reduce#2 type=<U>(this: this, (U, T#1) => U, U) => U
    /// @type.symbol symbol=It.reduce.U source=U type=U
    /// @type.symbol symbol=It.reduce.this#2 source=this type=this
    /// @type.symbol symbol=It.reduce.reduce#2 source="reduce: (accumulator: U, value: T) => U" type=(U, T#1) => U
    /// @type.symbol symbol=It.reduce.accumulator#2 source="accumulator: U" type=U
    /// @resolution.name source=U target=It.reduce.U
    /// @type.symbol symbol=It.reduce.value#2 source="value: T" type=T#1
    /// @resolution.name source=T target=It.T
    /// @resolution.name source=U target=It.reduce.U
    /// @type.symbol symbol=It.reduce.initial source="initial: U" type=U
    /// @resolution.name source=U target=It.reduce.U
    /// @resolution.name source=U target=It.reduce.U

        todo("reduce")
        /// @type.node source="todo(\"reduce\")" type=never
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"reduce\")" parameters=(string | undefined) arguments=(provided("reduce") as string | undefined) return=never kind=symbol target=todo
        /// @type.node source="\"reduce\"" type="reduce"

    }
}

struct Wrap<T> {
/// @generic.template symbol=Wrap parameters=(out T#2)
/// @type.symbol symbol=Wrap type=Wrap
/// @definition.struct symbol=Wrap template=(out T#2)
/// @definition.field symbol=Wrap.value source="value: T" key=value type=T#2
/// @type.symbol symbol=Wrap.T source=T type=T#2

    value: T;
    /// @type.symbol symbol=Wrap.value source="value: T" type=T#2
    /// @resolution.name source=T target=Wrap.T

}

function wrap<T>(value: T): Wrap<T> {
/// @generic.template symbol=wrap parameters=(T#3)
/// @type.symbol symbol=wrap type=<T#3>(T#3) => Wrap<T#3>
/// @generic.instance id=Wrap<T#3> template=Wrap arguments=(T#3)
/// @type.symbol symbol=wrap.T source=T type=T#3
/// @type.symbol symbol=wrap.value source="value: T" type=T#3
/// @resolution.name source=T target=wrap.T
/// @resolution.name source=Wrap target=Wrap
/// @resolution.name source=T target=wrap.T

    Wrap { value }
    /// @type.node source="Wrap { value }" type=Wrap<T#3>
    /// @resolution.name source=Wrap target=Wrap
    /// @resolution.name source=value target=wrap.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=wrap.value

}

function sum(values: It<int32>): Wrap<int32> {
/// @type.symbol symbol=sum type=(It<int32, void>) => Wrap<int32>
/// @generic.instance id="It<int32, void>" template=It arguments=(int32, void)
/// @generic.instance id=Wrap<int32> template=Wrap arguments=(int32)
/// @type.symbol symbol=sum.values source="values: It<int32>" type=It<int32, void>
/// @resolution.name source=It target=It
/// @resolution.name source=Wrap target=Wrap

    return values.reduce((result, value) => {
    /// @type.node source=values.reduce type=(this: It<int32, void>, (int32, int32) => int32) => int32 & <U>(this: It<int32, void>, (U, int32) => U, U) => U
    /// @type.node type=Wrap<int32>
    /// @resolution.name source=values target=sum.values
    /// @resolution.member source=values.reduce receiver=It<int32, void> type=(this: It<int32, void>, (int32, int32) => int32) => int32 & <U>(this: It<int32, void>, (U, int32) => U, U) => U kind=overload-set targets=[It.reduce#1, It.reduce#2]
    /// @resolution.call parameters=((Wrap<int32>, int32) => Wrap<int32>, Wrap<int32>) arguments=(provided(argument) as (Wrap<int32>, int32) => Wrap<int32>, provided(wrap(0)) as Wrap<int32>) return=Wrap<int32> kind=dynamic target=It.reduce#2 receiver=It<int32, void> constraint=It<int32, void> generic_arguments=(int32, void, Wrap<int32>)
    /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values root=sum.values
    /// @generic.instantiation id="It.reduce#1<int32, void>" template=It.reduce#1 arguments=(int32, void)
    /// @generic.instantiation id="It.reduce#2<int32, void>" template=It.reduce#2 arguments=(int32, void)
    /// @type.symbol symbol=sum.symbol30 type=Function<(Wrap<int32>, int32), Wrap<int32>, "readonly">
    /// @type.node type=Function<(Wrap<int32>, int32), Wrap<int32>, "readonly">
    /// @type.symbol symbol=sum.symbol30.result source=result type=Wrap<int32>
    /// @type.symbol symbol=sum.symbol30.value source=value type=int32

        wrap(value)
        /// @type.node source=wrap(value) type=Wrap<int32>
        /// @resolution.name source=wrap target=wrap
        /// @resolution.call source=wrap(value) parameters=(int32) arguments=(provided(value) as int32) return=Wrap<int32> kind=symbol target=wrap instance=wrap<int32>
        /// @resolution.name source=value target=sum.symbol30.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=sum.symbol30.value

    }, wrap(0));
    /// @type.node source=wrap(0) type=Wrap<int32>
    /// @resolution.name source=wrap target=wrap
    /// @resolution.call source=wrap(0) parameters=(int32) arguments=(provided(0) as int32) return=Wrap<int32> kind=symbol target=wrap instance=wrap<int32>
    /// @generic.instantiation id=wrap<int32> template=wrap arguments=(int32)
    /// @generic.instance id=wrap<int32> template=wrap arguments=(int32)
    /// @type.node source=0 type=0

}
"#);
}

#[test]
fn test_expand_spread_arguments_in_rest_windows() {
    let session = TestSession::single(
        r#"
function total(...values: int32[]): int32 {
    return 0;
}

function append(values: int32[], more: ^int32[]): int32 {
    values.push(1, 2);
    values.push(...more);
    values.push(1, ...more, 3);

    return total(1, ...more, 3);
}
"#,
    );

    session.assert_dir("main.ds", DirRows::checked().with_node_types(), r#"
=== annotated ===
function total(...values: int32[]): int32 {
    return 0;
}

function append(values: int32[], more: ^int32[]): int32 {
    values.push<int32, "managed">(1, 2);
    values.push<int32, "managed">(...more);
    values.push<int32, "managed">(1, ...more, 3);

    return total(1, ...more, 3);
}

=== dir ===
function total(...values: int32[]): int32 {
/// @type.symbol symbol=total type=(...int32[]) => int32
/// @generic.instance id=Array<int32> template=Array arguments=(int32)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int32>> template=sliceAssumeInit arguments=(MaybeUninit<int32>)
/// @generic.instance id=sliceUninit<MaybeUninit<int32>> template=sliceUninit arguments=(MaybeUninit<int32>)
/// @type.symbol symbol=total.values source="...values: int32[]" type=int32[]

    return 0;
    /// @type.node source=0 type=0

}

function append(values: int32[], more: ^int32[]): int32 {
/// @type.symbol symbol=append type=(int32[], ^int32[]) => int32
/// @type.symbol symbol=append.values source="values: int32[]" type=int32[]
/// @type.symbol symbol=append.more source="more: ^int32[]" type=^int32[]

    values.push(1, 2);
    /// @type.node source="values.push(1, 2)" type=isize
    /// @type.node source=values.push type=<push.'a>(this: &push.'a int32[], ...int32[]) => isize
    /// @resolution.name source=values target=append.values
    /// @resolution.member source=values.push receiver=int32[] type=<push.'a>(this: &push.'a int32[], ...int32[]) => isize kind=symbol target_receiver=int32[] target=push
    /// @resolution.call source="values.push(1, 2)" parameters=(int32[]) arguments=(rest(provided(1) as int32, provided(2) as int32) pack=arrayFromOwnedSlice as int32) return=isize regions=("managed" & "local") kind=symbol target=push receiver=int32[] adjustments=(borrow(&'managed int32[])) instance="Array<int32>.<extension#6>.push<\"managed\" & \"local\">"
    /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values root=append.values
    /// @generic.instantiation id="push<int32, \"managed\" & \"local\">" template=push arguments=(int32, "managed" & "local")
    /// @generic.instantiation id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
    /// @generic.instantiation id=push<int32> template=push arguments=(int32)
    /// @generic.instance id="push<int32, \"bound0\" & \"local\">" template=push arguments=(int32, "bound0" & "local")
    /// @generic.instance id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
    /// @type.node source=1 type=1
    /// @type.node source=2 type=2

    values.push(...more);
    /// @type.node source=values.push type=<push.'a>(this: &push.'a int32[], ...int32[]) => isize
    /// @type.node source=values.push(...more) type=isize
    /// @resolution.name source=values target=append.values
    /// @resolution.member source=values.push receiver=int32[] type=<push.'a>(this: &push.'a int32[], ...int32[]) => isize kind=symbol target_receiver=int32[] target=push
    /// @resolution.call source=values.push(...more) parameters=(int32[]) arguments=(rest(spread(provided(...more) as ^int32[], iterator=iterator#1(parameters=(), arguments=(), return=Iterator<int32>), next=dynamic(Iterator<int32> as Iterator<int32>, Iterator.next)(parameters=(), arguments=(), return=IteratorResult<int32, void>, regions=("managed" & "local"))) as int32) pack=arrayFromOwnedSlice as int32) return=isize regions=("managed" & "local") kind=symbol target=push receiver=int32[] adjustments=(borrow(&'managed int32[])) instance="Array<int32>.<extension#6>.push<\"managed\" & \"local\">"
    /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values root=append.values
    /// @generic.instantiation id=iterator#1<int32> template=iterator#1 arguments=(int32)
    /// @generic.instance id="IteratorResult<int32, void>" template=IteratorResult arguments=(int32, void)
    /// @generic.instance id=Iterator<int32> template=Iterator arguments=(int32)
    /// @generic.instance id=IteratorReturn<void> template=IteratorReturn arguments=(void)
    /// @generic.instance id=IteratorYield<int32> template=IteratorYield arguments=(int32)
    /// @generic.instance id=iterator#1<int32> template=iterator#1 arguments=(int32)
    /// @resolution.name source=more target=append.more
    /// @resolution.place source=more placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=more root=append.more

    values.push(1, ...more, 3);
    /// @type.node source="values.push(1, ...more, 3)" type=isize
    /// @type.node source=values.push type=<push.'a>(this: &push.'a int32[], ...int32[]) => isize
    /// @resolution.name source=values target=append.values
    /// @resolution.member source=values.push receiver=int32[] type=<push.'a>(this: &push.'a int32[], ...int32[]) => isize kind=symbol target_receiver=int32[] target=push
    /// @resolution.call source="values.push(1, ...more, 3)" parameters=(int32[]) arguments=(rest(provided(1) as int32, spread(provided(...more) as ^int32[], iterator=iterator#1(parameters=(), arguments=(), return=Iterator<int32>), next=dynamic(Iterator<int32> as Iterator<int32>, Iterator.next)(parameters=(), arguments=(), return=IteratorResult<int32, void>, regions=("managed" & "local"))) as int32, provided(3) as int32) pack=arrayFromOwnedSlice as int32) return=isize regions=("managed" & "local") kind=symbol target=push receiver=int32[] adjustments=(borrow(&'managed int32[])) instance="Array<int32>.<extension#6>.push<\"managed\" & \"local\">"
    /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values root=append.values
    /// @type.node source=1 type=1
    /// @resolution.name source=more target=append.more
    /// @resolution.place source=more placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=more root=append.more
    /// @type.node source=3 type=3

    return total(1, ...more, 3);
    /// @type.node source="total(1, ...more, 3)" type=int32
    /// @resolution.name source=total target=total
    /// @resolution.call source="total(1, ...more, 3)" parameters=(int32[]) arguments=(rest(provided(1) as int32, spread(provided(...more) as ^int32[], iterator=iterator#1(parameters=(), arguments=(), return=Iterator<int32>), next=dynamic(Iterator<int32> as Iterator<int32>, Iterator.next)(parameters=(), arguments=(), return=IteratorResult<int32, void>, regions=("managed" & "local"))) as int32, provided(3) as int32) pack=arrayFromOwnedSlice as int32) return=int32 kind=symbol target=total
    /// @type.node source=1 type=1
    /// @resolution.name source=more target=append.more
    /// @resolution.place source=more placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=more root=append.more
    /// @type.node source=3 type=3

}
"#);
}

#[test]
fn test_reject_spread_arguments_at_positional_parameters() {
    let session = TestSession::single(
        r#"
function pick(first: int32): int32 {
    return first;
}

function fails(more: ^int32[]): int32 {
    return pick(...more);
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_node_types(),
        r#"
=== annotated ===
function pick(first: int32): int32 {
    return first;
}

function fails(more: ^int32[]): int32 {
    return pick(...more);
}

=== dir ===
function pick(first: int32): int32 {
/// @type.symbol symbol=pick type=(int32) => int32
/// @type.symbol symbol=pick.first source="first: int32" type=int32

    return first;
    /// @resolution.name source=first target=pick.first
    /// @resolution.place source=first placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=first root=pick.first

}

function fails(more: ^int32[]): int32 {
/// @type.symbol symbol=fails type=(^int32[]) => int32
/// @type.symbol symbol=fails.more source="more: ^int32[]" type=^int32[]

    return pick(...more);
    /// @type.node source=pick(...more) type=<error>
    /// @resolution.name source=pick target=pick
    /// @resolution.call source=pick(...more) parameters=(int32) arguments=(spread(provided(...more) as ^int32[], iterator=iterator#1(parameters=(), arguments=(), return=Iterator<int32>), next=dynamic(Iterator<int32> as Iterator<int32>, Iterator.next)(parameters=(), arguments=(), return=IteratorResult<int32, void>, regions=("managed" & "local"))) as int32) return=int32 kind=symbol target=pick
    /// @generic.instantiation id=iterator#1<int32> template=iterator#1 arguments=(int32)
    /// @resolution.name source=more target=fails.more
    /// @resolution.place source=more placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=more root=fails.more

}
"#,
        r#"
/// @diagnostic.error id=no-matching-call message="no overload matches arguments ('^int32[]')"
/// @diagnostic.label line=7 column=12 span="pick(...more)" line_source="return pick(...more);"
/// @diagnostic.note message="the candidate '(first: int32) => int32' does not apply"
"#,
    );
}

#[test]
fn test_project_associated_returns_through_bound_parameters() {
    let session = TestSession::single(
        r#"
newtype interface It<T> {
    type Return = void;

    next(this): this.Return {
        todo("next")
    }
}

struct Wrap<I> {
    source: I;
}

extension<T, I: It<T>> of Wrap<I> implements It<T> {
    type Return = I.Return;

    next(): I.Return {
        todo("next")
    }
}

struct Counter {
    count: int32;
}

extension of Counter implements It<int32> {
    type Return = boolean;

    next(): boolean {
        todo("next")
    }
}

function finish(wrapped: Wrap<Counter>): boolean {
    return wrapped.next();
}
"#,
    );

    session.assert_dir("main.ds", DirRows::checked().with_node_types(), r#"
=== annotated ===
newtype interface It<T> {
    type Return = void;

    next(this): this.Return {
        todo("next" as string | undefined)
    }
}

struct Wrap<out I> {
    source: I;
}

extension<T, I: It<T>> of Wrap<I> implements It<T> {
    type Return = I.Return;

    next(): I.Return {
        todo("next" as string | undefined)
    }
}

struct Counter {
    count: int32;
}

extension of Counter implements It<int32> {
    type Return = boolean;

    next(): boolean {
        todo("next" as string | undefined)
    }
}

function finish(wrapped: Wrap<Counter>): boolean {
    return wrapped.next<int32, Counter, "frame">();
}

=== dir ===
newtype interface It<T> {
/// @generic.template symbol=It parameters=(T#1, this: It<T#1>)
/// @type.symbol symbol=It type=It
/// @definition.interface symbol=It template=(T#1, this: It<T#1>) nominal=true
/// @definition.where symbol=It relation=satisfies left=this right=It<T#1>
/// @definition.associated.type symbol=It.Return source="type Return = void" key=Return value=void
/// @definition.method symbol=It.next slot=next type=(this: this) => this.Return
/// @type.symbol symbol=It.T source=T type=T#1

    type Return = void;
    /// @type.symbol symbol=It.Return source="type Return = void" type=void

    next(this): this.Return {
    /// @type.symbol symbol=It.next type=(this: this) => this.Return
    /// @type.symbol symbol=It.next.this source=this type=this
    /// @resolution.name source=this.Return target=It.Return

        todo("next")
        /// @type.node source="todo(\"next\")" type=never
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"next\")" parameters=(string | undefined) arguments=(provided("next") as string | undefined) return=never kind=symbol target=todo
        /// @type.node source="\"next\"" type="next"

    }
}

struct Wrap<I> {
/// @generic.template symbol=Wrap parameters=(out I#1)
/// @type.symbol symbol=Wrap type=Wrap
/// @definition.struct symbol=Wrap template=(out I#1)
/// @definition.field symbol=Wrap.source source="source: I" key=source type=I#1
/// @type.symbol symbol=Wrap.I source=I type=I#1

    source: I;
    /// @type.symbol symbol=Wrap.source source="source: I" type=I#1
    /// @resolution.name source=I target=Wrap.I

}

extension<T, I: It<T>> of Wrap<I> implements It<T> {
/// @generic.template symbol=<module>#2 parameters=(T#2, I#2: It<T#2>)
/// @generic.instance id=It<T#2> template=It arguments=(T#2)
/// @generic.instance id=Wrap<I#2> template=Wrap arguments=(I#2)
/// @definition.extension symbol=<module>#2 form=local target=Wrap<I#2>
/// @definition.implements symbol=<module>#2 source=It<T> target=It<T#2>
/// @definition.associated.type symbol=Return#1 source="type Return = I.Return" key=Return value=I#2.Return
/// @definition.method symbol=next#1 slot=next type=<next#1.'a>(this: &next#1.'a readonly Wrap<I#2>) => I#2.Return
/// @definition.conformance symbol=<module>#2 member=Return#1 requirement=It.Return
/// @definition.conformance symbol=<module>#2 member=next#1 requirement=It.next
/// @type.symbol symbol=T source=T type=T#2
/// @type.symbol symbol=I source="I: It<T>" type=I#2
/// @resolution.name source=It target=It
/// @resolution.name source=T target=T
/// @resolution.name source=Wrap target=Wrap
/// @resolution.name source=I target=I
/// @resolution.name source=It target=It
/// @resolution.name source=T target=T

    type Return = I.Return;
    /// @type.symbol symbol=Return#1 source="type Return = I.Return" type=I#2.Return
    /// @resolution.name source=I.Return target=I
    /// @resolution.path source=I.Return index=1 target=It.Return

    next(): I.Return {
    /// @generic.template symbol=next#1 parent=template#2 parameters=('a)
    /// @type.symbol symbol=next#1 type=<next#1.'a>(this: &next#1.'a readonly Wrap<I#2>) => I#2.Return
    /// @type.symbol symbol=next.this#1 type=&next#1.'a readonly Wrap<I#2>
    /// @resolution.name source=I.Return target=I
    /// @resolution.path source=I.Return index=1 target=It.Return

        todo("next")
        /// @type.node source="todo(\"next\")" type=never
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"next\")" parameters=(string | undefined) arguments=(provided("next") as string | undefined) return=never kind=symbol target=todo
        /// @type.node source="\"next\"" type="next"

    }
}

struct Counter {
/// @type.symbol symbol=Counter type=Counter
/// @definition.struct symbol=Counter
/// @definition.field symbol=Counter.count source="count: int32" key=count type=int32

    count: int32;
    /// @type.symbol symbol=Counter.count source="count: int32" type=int32

}

extension of Counter implements It<int32> {
/// @generic.instance id=It<int32> template=It arguments=(int32)
/// @definition.extension symbol=<module>#3 form=local target=Counter
/// @definition.implements symbol=<module>#3 source=It<int32> target=It<int32>
/// @definition.associated.type symbol=Return#2 source="type Return = boolean" key=Return value=boolean
/// @definition.method symbol=next#2 slot=next type=<next#2.'a>(this: &next#2.'a readonly Counter) => boolean
/// @definition.conformance symbol=<module>#3 member=Return#2 requirement=It.Return
/// @definition.conformance symbol=<module>#3 member=next#2 requirement=It.next
/// @resolution.name source=Counter target=Counter
/// @resolution.name source=It target=It

    type Return = boolean;
    /// @type.symbol symbol=Return#2 source="type Return = boolean" type=boolean

    next(): boolean {
    /// @generic.template symbol=next#2 parent=template#3 parameters=('a)
    /// @type.symbol symbol=next#2 type=<next#2.'a>(this: &next#2.'a readonly Counter) => boolean
    /// @type.symbol symbol=next.this#2 type=&next#2.'a readonly Counter

        todo("next")
        /// @type.node source="todo(\"next\")" type=never
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"next\")" parameters=(string | undefined) arguments=(provided("next") as string | undefined) return=never kind=symbol target=todo
        /// @type.node source="\"next\"" type="next"

    }
}

function finish(wrapped: Wrap<Counter>): boolean {
/// @type.symbol symbol=finish type=(Wrap<Counter>) => boolean
/// @generic.instance id=Wrap<Counter> template=Wrap arguments=(Counter)
/// @type.symbol symbol=finish.wrapped source="wrapped: Wrap<Counter>" type=Wrap<Counter>
/// @resolution.name source=Wrap target=Wrap
/// @resolution.name source=Counter target=Counter

    return wrapped.next();
    /// @type.node source=wrapped.next type=<next#1.'a>(this: &next#1.'a readonly Wrap<Counter>) => boolean
    /// @type.node source=wrapped.next() type=boolean
    /// @resolution.name source=wrapped target=finish.wrapped
    /// @resolution.member source=wrapped.next receiver=Wrap<Counter> type=<next#1.'a>(this: &next#1.'a readonly Wrap<Counter>) => boolean kind=symbol target_receiver=Wrap<Counter> target=next#1
    /// @resolution.call source=wrapped.next() parameters=() return=boolean regions=("frame" & "local") kind=symbol target=next#1 receiver=Wrap<Counter> adjustments=(borrow(&'frame readonly Wrap<Counter>)) instance="Wrap<Counter>.<extension#1>.next#1<\"frame\" & \"local\">"
    /// @resolution.place source=wrapped placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=wrapped root=finish.wrapped
    /// @generic.instantiation id="next#1<int32, Counter, \"frame\" & \"local\">" template=next#1 arguments=(int32, Counter, "frame" & "local")
    /// @generic.instantiation id="next#1<int32, Counter>" template=next#1 arguments=(int32, Counter)
    /// @generic.instance id="next#1<int32, Counter, \"bound0\" & \"local\">" template=next#1 arguments=(int32, Counter, "bound0" & "local")

}
"#);
}

/// Spread elements convert into the rest parameter as values, taking views the arm declares.
#[test]
fn test_spread_arrays_into_a_readonly_view_rest_parameter() {
    let session = TestSession::single(
        r#"
declare function take(...items: (int32 | readonly int32[])[]): void;

function forward(values: int32[][]): void {
    take(...values);
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function take(...items: (int32 | readonly int32[])[]): void;

function forward(values: int32[][]): void {
    take(...values);
}

=== dir ===
declare function take(...items: (int32 | readonly int32[])[]): void;
/// @type.symbol symbol=take source="declare function take(...items: (int32 | readonly int32[])[]): void" type=(...int32 | readonly int32[][]) => void

function forward(values: int32[][]): void {
/// @type.symbol symbol=forward type=(int32[][]) => void
/// @type.symbol symbol=forward.values source="values: int32[][]" type=int32[][]

    take(...values);
    /// @resolution.name source=take target=take
    /// @resolution.call source=take(...values) parameters=(int32 | readonly int32[][]) arguments=(rest(spread(provided(...values) as int32[][], iterator=iterator#2(parameters=(), arguments=(), return=Iterator<int32[]>), next=dynamic(Iterator<int32[]> as Iterator<int32[]>, Iterator.next)(parameters=(), arguments=(), return=IteratorResult<int32[], void>, regions=("managed" & "local"))) as int32 | readonly int32[]) pack=arrayFromOwnedSlice as int32 | readonly int32[]) return=void kind=symbol target=take
    /// @generic.instantiation id="arrayFromOwnedSlice<int32 | readonly int32[]>" template=arrayFromOwnedSlice arguments=(int32 | readonly int32[])
    /// @generic.instantiation id=iterator#2<int32[]> template=iterator#2 arguments=(int32[])
    /// @resolution.name source=values target=forward.values
    /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values root=forward.values

}
"#,
        r#"
"#,
    );
}
