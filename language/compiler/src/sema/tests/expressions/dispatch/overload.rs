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
/// @type.symbol symbol=parse.value#1 source="value: string" type=string

declare function parse(value: int32): int32;
/// @type.symbol symbol=parse#2 source="declare function parse(value: int32): int32" type=(int32) => int32
/// @type.symbol symbol=parse.value#2 source="value: int32" type=int32

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
/// @type.symbol symbol=parse.value source="value: string" type=string
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
/// @definition.method symbol=capacity source="get capacity(): usize" slot=capacity role=getter type=<capacity.'a>(this: &capacity.'a readonly this) => usize
/// @definition.method symbol=trailing source="trailing(): usize" slot=trailing type=<trailing.'a>(this: &trailing.'a exclusive this) => usize
/// @resolution.name source=Buffer target=Buffer

    @intrinsic("buffer.capacity")
    /// @resolution.name source=intrinsic target=decorator.intrinsic.intrinsic

    get capacity(): usize;
    /// @generic.template symbol=capacity parameters=('a)
    /// @type.symbol symbol=capacity source="get capacity(): usize" type=<capacity.'a>(this: &capacity.'a readonly this) => usize

    trailing(): usize;
    /// @generic.template symbol=trailing parameters=('a)
    /// @type.symbol symbol=trailing source="trailing(): usize" type=<trailing.'a>(this: &trailing.'a exclusive this) => usize

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
fn test_check_retains_attempted_call_bindings_on_rejection() {
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

const text: (arg0: string) => string = render;

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
/// @type.symbol symbol=text source=text type=Function<(string,), string>
/// @resolution.pattern source=text kind=binding target=text
/// @type.symbol symbol=value source="value: string" type=string
/// @resolution.name source=render target=[render#1, render#2]
/// @resolution.function source=render type=(string) => string target=render#2
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

const chosen: (arg0: boolean) => boolean = render;

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
/// @type.symbol symbol=chosen source=chosen type=Function<(boolean,), boolean>
/// @resolution.pattern source=chosen kind=binding target=chosen
/// @type.symbol symbol=value source="value: boolean" type=boolean
/// @resolution.name source=render target=[render#1, render#2]
"#,
        r#"
/// @diagnostic.error id=ambiguous-overload message="overload 'render' is ambiguous without a call"
/// @diagnostic.label line=10 column=45 span="render" line_source="const chosen: (value: boolean) => boolean = render;"
"#,
    );
}
