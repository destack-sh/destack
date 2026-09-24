use crate::tests::{DirRows, TestSession};

#[test]
fn test_apply_decorator_to_interface_method() {
    let session = TestSession::single(
        r#"
newtype mark = (string,);

interface Reader {
    @mark("checked")
    read(@mark("parameter") value: string): string;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types().with_decorators(),
        r#"
=== annotated ===
newtype mark = (string,);

interface Reader {
    @mark("checked")
    read(@mark("parameter") value: string): string;
}

=== dir ===
newtype mark = (string,);
/// @type.symbol symbol=mark source="newtype mark = (string,)" type=mark
/// @definition.newtype symbol=mark source="newtype mark = (string,)" backing=(string,) constructors=[(string) => mark]

interface Reader {
/// @generic.template symbol=Reader parameters=(this: Reader)
/// @type.symbol symbol=Reader type=Reader
/// @definition.interface symbol=Reader template=(this: Reader)
/// @definition.where symbol=Reader relation=satisfies left=this right=Reader
/// @definition.method symbol=Reader.read source="read(@mark(\"parameter\") value: string): string" slot=read type=(string) => string

    @mark("checked")
    /// @decorator.node source="@mark(\"checked\")" owner="read(@mark(\"parameter\") value: string): string" expression=mark target=mark type=mark kind=newtype parameters=(string) arguments=(provided("checked") as string) newtype=mark backing=(string,) value="mark(\"checked\")"
    /// @type.node source=mark type=mark
    /// @resolution.name source=mark target=mark
    /// @type.node source="\"checked\"" type="checked"

    read(@mark("parameter") value: string): string;
    /// @type.symbol symbol=Reader.read source="read(@mark(\"parameter\") value: string): string" type=(string) => string
    /// @decorator.node source="@mark(\"parameter\")" owner="value: string" expression=mark target=mark type=mark kind=newtype parameters=(string) arguments=(provided("parameter") as string) newtype=mark backing=(string,) value="mark(\"parameter\")"
    /// @type.node source=mark type=mark
    /// @resolution.name source=mark target=mark
    /// @type.node source="\"parameter\"" type="parameter"
    /// @type.symbol symbol=Reader.read.value source="value: string" type=string

}
"#,
    );
}

#[test]
fn test_decorator_resolves_outside_declaration_scope() {
    let session = TestSession::single(
        r#"
newtype mark = (string,);

@mark("checked")
declare function read(mark: int32): void;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types().with_decorators(),
        r#"
=== annotated ===
newtype mark = (string,);

@mark("checked")
declare function read(mark: int32): void;

=== dir ===
newtype mark = (string,);
/// @type.symbol symbol=mark source="newtype mark = (string,)" type=mark
/// @definition.newtype symbol=mark source="newtype mark = (string,)" backing=(string,) constructors=[(string) => mark]

@mark("checked")
/// @decorator.node source="@mark(\"checked\")" owner="declare function read(mark: int32): void" expression=mark target=mark type=mark kind=newtype parameters=(string) arguments=(provided("checked") as string) newtype=mark backing=(string,) value="mark(\"checked\")"
/// @type.node source=mark type=mark
/// @resolution.name source=mark target=mark
/// @type.node source="\"checked\"" type="checked"

declare function read(mark: int32): void;
/// @type.symbol symbol=read source="declare function read(mark: int32): void" type=(int32) => void
"#,
    );
}

#[test]
fn test_member_access_decorator_reports_error() {
    let session = TestSession::single(
        r#"
const subject = 1;

@subject.field
const value = 1;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const subject: 1 = 1;

@subject.field
const value: 1 = 1;

=== dir ===
const subject = 1;
/// @type.symbol symbol=subject source=subject type=1
/// @resolution.pattern source=subject kind=binding target=subject
/// @type.node source=1 type=1

@subject.field
const value = 1;
/// @type.symbol symbol=value source=value type=1
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=1 type=1
"#,
        r#"
/// @diagnostic.error id=invalid-decorator-target message="decorator must name a newtype declaration"
/// @diagnostic.label line=4 column=10 span="field" line_source="@subject.field"
"#,
    );
}

#[test]
fn test_decorator_preserves_nested_newtype_value() {
    let session = TestSession::single(
        r#"
newtype Payload = { reason: string };
newtype mark = (Payload,);

@mark(Payload({ reason: "intentional" }))
const value = 1;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types().with_decorators(),
        r#"
=== annotated ===
newtype Payload = { reason: string };
newtype mark = (Payload,);

@mark(Payload({ reason: "intentional" }))
const value: 1 = 1;

=== dir ===
newtype Payload = { reason: string };
/// @type.symbol symbol=Payload source="newtype Payload = { reason: string }" type=Payload
/// @definition.newtype symbol=Payload source="newtype Payload = { reason: string }" backing={ reason: string } constructors=[({ reason: string }) => Payload]
/// @type.symbol symbol=Payload.reason source="reason: string" type=string

newtype mark = (Payload,);
/// @type.symbol symbol=mark source="newtype mark = (Payload,)" type=mark
/// @definition.newtype symbol=mark source="newtype mark = (Payload,)" backing=(Payload,) constructors=[(Payload) => mark]
/// @resolution.name source=Payload target=Payload

@mark(Payload({ reason: "intentional" }))
/// @decorator.node source="@mark(Payload({ reason: \"intentional\" }))" owner="const value = 1" expression=mark target=mark type=mark kind=newtype parameters=(Payload) arguments=(provided(Payload({ reason: "intentional" })) as Payload) newtype=mark backing=(Payload,) value="mark(Payload({ reason: \"intentional\" }))"
/// @type.node source=mark type=mark
/// @resolution.name source=mark target=mark
/// @type.node source="Payload({ reason: \"intentional\" })" type=Payload
/// @type.node source=Payload type=Payload
/// @resolution.name source=Payload target=Payload
/// @resolution.construct source="Payload({ reason: \"intentional\" })" parameters=({ reason: string }) arguments=(provided({ reason: "intentional" }) as { reason: string }) return=Payload kind=newtype target=Payload backing={ reason: string }
/// @type.node source={ reason: "intentional" } type={ reason: string }
/// @type.node source="\"intentional\"" type="intentional"

const value = 1;
/// @type.symbol symbol=value source=value type=1
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=1 type=1
"#,
    );
}

/// Flatten static object spreads into the checked decorator value.
#[test]
fn test_flatten_static_object_spread() {
    let session = TestSession::single(
        r#"
newtype mark = ({ reason: string },);

@mark({ reason: "direct reason", ...{ reason: "spread reason" } })
const value = 1;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types().with_decorators(),
        r#"
=== annotated ===
newtype mark = ({ reason: string },);

@mark({ reason: "direct reason", ...{ reason: "spread reason" } })
const value: 1 = 1;

=== dir ===
newtype mark = ({ reason: string },);
/// @type.symbol symbol=mark source="newtype mark = ({ reason: string },)" type=mark
/// @definition.newtype symbol=mark source="newtype mark = ({ reason: string },)" backing=({ reason: string },) constructors=[({ reason: string }) => mark]
/// @type.symbol symbol=mark.reason source="reason: string" type=string

@mark({ reason: "direct reason", ...{ reason: "spread reason" } })
/// @decorator.node source="@mark({ reason: \"direct reason\", ...{ reason: \"spread reason\" } })" owner="const value = 1" expression=mark target=mark type=mark kind=newtype parameters=({ reason: string }) arguments=(provided({ reason: "direct reason", ...{ reason: "spread reason" } }) as { reason: string }) newtype=mark backing=({ reason: string },) value="mark({ reason: \"spread reason\" })"
/// @type.node source=mark type=mark
/// @resolution.name source=mark target=mark
/// @type.node source={ reason: "direct reason", ...{ reason: "spread reason" } } type={ reason: string }
/// @type.node source="\"direct reason\"" type="direct reason"
/// @type.node source={ reason: "spread reason" } type={ reason: string }
/// @type.node source="\"spread reason\"" type="spread reason"

const value = 1;
/// @type.symbol symbol=value source=value type=1
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=1 type=1
"#,
    );
}

#[test]
fn test_apply_decorator_to_argument_inside_function_body() {
    let session = TestSession::single(
        r#"
newtype mark = (string,);

class Sink {
    constructor(value: int32) {}
}

declare function consume(value: int32): void;

function run(): void {
    consume(@mark("call") 1);
    const sink = new Sink(@mark("construct") 2);
    const array = [@mark("array") 3];
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types().with_decorators(),
        r#"
=== annotated ===
newtype mark = (string,);

class Sink {
    constructor(value: int32) {}
}

declare function consume(value: int32): void;

function run(): void {
    consume(@mark("call") 1);
    const sink: Sink = new Sink(@mark("construct") 2);
    const array: int64[] = [@mark("array") 3,];
}

=== dir ===
newtype mark = (string,);
/// @type.symbol symbol=mark source="newtype mark = (string,)" type=mark
/// @definition.newtype symbol=mark source="newtype mark = (string,)" backing=(string,) constructors=[(string) => mark]

class Sink {
/// @type.symbol symbol=Sink type=typeof Sink
/// @definition.class symbol=Sink
/// @definition.method symbol=Sink.constructor source="constructor(value: int32) {}" slot=constructor role=constructor type=(this: &'managed Sink, int32) => Sink

    constructor(value: int32) {}
    /// @type.symbol symbol=Sink.constructor source="constructor(value: int32) {}" type=(this: &'managed Sink, int32) => Sink
    /// @type.symbol symbol=Sink.constructor.this type=&'managed Sink
    /// @type.symbol symbol=Sink.constructor.value source="value: int32" type=int32

}

declare function consume(value: int32): void;
/// @type.symbol symbol=consume source="declare function consume(value: int32): void" type=(int32) => void

function run(): void {
/// @type.symbol symbol=run type=() => void

    consume(@mark("call") 1);
    /// @type.node source="consume(@mark(\"call\") 1)" type=void
    /// @type.node source=consume type=(int32) => void
    /// @resolution.name source=consume target=consume
    /// @resolution.call source="consume(@mark(\"call\") 1)" parameters=(int32) arguments=(provided(@mark("call") 1) as int32) return=void kind=symbol target=consume
    /// @decorator.node source="@mark(\"call\")" owner="@mark(\"call\") 1" expression=mark target=mark type=mark kind=newtype parameters=(string) arguments=(provided("call") as string) newtype=mark backing=(string,) value="mark(\"call\")"
    /// @type.node source=mark type=mark
    /// @resolution.name source=mark target=mark
    /// @type.node source="\"call\"" type="call"
    /// @type.node source=1 type=1

    const sink = new Sink(@mark("construct") 2);
    /// @type.symbol symbol=run.sink source=sink type=Sink
    /// @resolution.pattern source=sink kind=binding target=run.sink
    /// @type.node source="new Sink(@mark(\"construct\") 2)" type=Sink
    /// @resolution.construct source="new Sink(@mark(\"construct\") 2)" parameters=(int32) arguments=(provided(@mark("construct") 2) as int32) return=Sink kind=class target=Sink constructor=Sink.constructor
    /// @type.node source=Sink type=typeof Sink
    /// @resolution.name source=Sink target=Sink
    /// @decorator.node source="@mark(\"construct\")" owner="@mark(\"construct\") 2" expression=mark target=mark type=mark kind=newtype parameters=(string) arguments=(provided("construct") as string) newtype=mark backing=(string,) value="mark(\"construct\")"
    /// @type.node source=mark type=mark
    /// @resolution.name source=mark target=mark
    /// @type.node source="\"construct\"" type="construct"
    /// @type.node source=2 type=2

    const array = [@mark("array") 3];
    /// @type.symbol symbol=run.array source=array type=int64[]
    /// @resolution.pattern source=array kind=binding target=run.array
    /// @generic.instance id=Array<int64> template=Array arguments=(int64)
    /// @generic.instance id=sliceAssumeInit<MaybeUninit<int64>> template=sliceAssumeInit arguments=(MaybeUninit<int64>)
    /// @generic.instance id=sliceUninit<MaybeUninit<int64>> template=sliceUninit arguments=(MaybeUninit<int64>)
    /// @type.node source=[@mark("array") 3] type=int64[]
    /// @resolution.call source=[@mark("array") 3] parameters=(^Slice<int64>) arguments=(rest(provided(@mark("array") 3) as int64) as int64) return=int64[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int64>
    /// @generic.instantiation id=arrayFromOwnedSlice<int64> template=arrayFromOwnedSlice arguments=(int64)
    /// @generic.instance id=arrayFromOwnedSlice<int64> template=arrayFromOwnedSlice arguments=(int64)
    /// @decorator.node source="@mark(\"array\")" owner="@mark(\"array\") 3" expression=mark target=mark type=mark kind=newtype parameters=(string) arguments=(provided("array") as string) newtype=mark backing=(string,) value="mark(\"array\")"
    /// @type.node source=mark type=mark
    /// @resolution.name source=mark target=mark
    /// @type.node source="\"array\"" type="array"
    /// @type.node source=3 type=3

}
"#,
    );
}

#[test]
fn test_apply_decorator_to_tuple_element_and_interpolation() {
    let session = TestSession::single(
        r#"
newtype mark = (string,);

function run(): void {
    const pair = (@mark("tuple") 4, 5);
    const text = `value ${@mark("interpolation") 6}`;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types().with_decorators(),
        r#"
=== annotated ===
newtype mark = (string,);

function run(): void {
    const pair: (int64, int64) = (
        @mark("tuple") 4,
        5,
    );
    const text: string = `value ${@mark("interpolation") 6}`;
}

=== dir ===
newtype mark = (string,);
/// @type.symbol symbol=mark source="newtype mark = (string,)" type=mark
/// @definition.newtype symbol=mark source="newtype mark = (string,)" backing=(string,) constructors=[(string) => mark]

function run(): void {
/// @type.symbol symbol=run type=() => void

    const pair = (@mark("tuple") 4, 5);
    /// @type.symbol symbol=run.pair source=pair type=(int64, int64)
    /// @resolution.pattern source=pair kind=binding target=run.pair
    /// @type.node source=(@mark("tuple") 4, 5) type=(int64, int64)
    /// @decorator.node source="@mark(\"tuple\")" owner="@mark(\"tuple\") 4" expression=mark target=mark type=mark kind=newtype parameters=(string) arguments=(provided("tuple") as string) newtype=mark backing=(string,) value="mark(\"tuple\")"
    /// @type.node source=mark type=mark
    /// @resolution.name source=mark target=mark
    /// @type.node source="\"tuple\"" type="tuple"
    /// @type.node source=4 type=4
    /// @type.node source=5 type=5

    const text = `value ${@mark("interpolation") 6}`;
    /// @type.symbol symbol=run.text source=text type=string
    /// @resolution.pattern source=text kind=binding target=run.text
    /// @type.node source="`value ${@mark(\"interpolation\") 6}`" type=string
    /// @resolution.template source="`value ${@mark(\"interpolation\") 6}`" spans=[Display.display(parameters=(), arguments=(), return=^string, regions=("frame" & "local"))] build="stringFromTemplate(parameters=(&'frame readonly Slice<string>, &'frame readonly Slice<string>), arguments=(supplied(0) as &'frame readonly Slice<string>, supplied(1) as &'frame readonly Slice<string>), return=string, regions=(\"frame\", \"frame\"))"
    /// @generic.instantiation id="Display.display<int64, \"frame\" & \"local\">" template=Display.display arguments=("frame" & "local")
    /// @generic.instantiation id="stringFromTemplate<\"frame\", \"frame\">" template=stringFromTemplate arguments=("frame", "frame")
    /// @generic.instance id="stringFromTemplate<\"frame\", \"frame\">" template=stringFromTemplate arguments=("frame", "frame")
    /// @decorator.node source="@mark(\"interpolation\")" owner="@mark(\"interpolation\") 6" expression=mark target=mark type=mark kind=newtype parameters=(string) arguments=(provided("interpolation") as string) newtype=mark backing=(string,) value="mark(\"interpolation\")"
    /// @type.node source=mark type=mark
    /// @resolution.name source=mark target=mark
    /// @type.node source="\"interpolation\"" type="interpolation"
    /// @type.node source=6 type=6

}
"#,
    );
}

#[test]
fn test_drop_statically_absent_tuple_element() {
    let session = TestSession::single(
        r#"
function run(): void {
    const pair = (1, @if(false) 2);
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types().with_decorators(),
        r#"
=== annotated ===
function run(): void {
    const pair: (int64,) = (
        1,
        @if(false) 2,
    );
}

=== dir ===
function run(): void {
/// @type.symbol symbol=run type=() => void

    const pair = (1, @if(false) 2);
    /// @type.symbol symbol=run.pair source=pair type=(int64,)
    /// @resolution.pattern source=pair kind=binding target=run.pair
    /// @type.node source=(1, @if(false) 2) type=(int64,)
    /// @type.node source=1 type=1

}
"#,
        r#"
"#,
    );
}

#[test]
fn test_drop_statically_absent_call_argument() {
    let session = TestSession::single(
        r#"
declare function consume(first: int32, second?: int32): void;
declare function require(first: int32): void;

function run(): void {
    consume(1, @if(false) 2);
    require(@if(false) 1);
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types().with_decorators(),
        r#"
=== annotated ===
declare function consume(first: int32, second?: int32): void;
declare function require(first: int32): void;

function run(): void {
    consume(1, @if(false) 2);
    require(@if(false) 1);
}

=== dir ===
declare function consume(first: int32, second?: int32): void;
/// @type.symbol symbol=consume source="declare function consume(first: int32, second?: int32): void" type=(int32, int32 | undefined?) => void

declare function require(first: int32): void;
/// @type.symbol symbol=require source="declare function require(first: int32): void" type=(int32) => void

function run(): void {
/// @type.symbol symbol=run type=() => void

    consume(1, @if(false) 2);
    /// @type.node source="consume(1, @if(false) 2)" type=void
    /// @type.node source=consume type=(int32, int32 | undefined?) => void
    /// @resolution.name source=consume target=consume
    /// @resolution.call source="consume(1, @if(false) 2)" parameters=(int32, int32 | undefined) arguments=(provided(1) as int32, omitted as int32 | undefined) return=void kind=symbol target=consume
    /// @type.node source=1 type=1

    require(@if(false) 1);
    /// @type.node source="require(@if(false) 1)" type=<error>
    /// @resolution.name source=require target=require
    /// @resolution.call source="require(@if(false) 1)" parameters=(int32) arguments=(omitted as int32) return=void kind=symbol target=require

}
"#,
        r#"
/// @diagnostic.error id=wrong-argument-count message="expected 1 argument, but got 0 argument(s)"
/// @diagnostic.label line=7 column=5 span="require(@if(false) 1)" line_source="require(@if(false) 1);"
"#,
    );
}

#[test]
fn test_reject_ambiguous_decorator_backing() {
    let session = TestSession::single(
        r#"
newtype mark = (string,) | ("value",);

@mark("value")
const value = 1;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
newtype mark = (string,) | ("value",);

@mark("value")
const value: 1 = 1;

=== dir ===
newtype mark = (string,) | ("value",);
/// @type.symbol symbol=mark source="newtype mark = (string,) | (\"value\",)" type=mark
/// @definition.newtype symbol=mark source="newtype mark = (string,) | (\"value\",)" backing=(string,) | ("value",) constructors=[(string) => mark, ("value") => mark, ((string,) | ("value",)) => mark]

@mark("value")
/// @type.node source=mark type=mark
/// @resolution.name source=mark target=mark
/// @type.node source="\"value\"" type="value"

const value = 1;
/// @type.symbol symbol=value source=value type=1
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=1 type=1
"#,
        r#"
/// @diagnostic.error id=ambiguous-decorator message="decorator arguments must select exactly one newtype backing"
/// @diagnostic.label line=4 column=2 span="mark(\"value\")" line_source="@mark(\"value\")"
"#,
    );
}

#[test]
fn test_infer_decorator_generic_arguments() {
    let session = TestSession::single(
        r#"
newtype mark<T> = (T,);

@mark(1)
const value = 1;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types().with_decorators(),
        r#"
=== annotated ===
newtype mark<out T> = (T,);

@mark(1)
const value: 1 = 1;

=== dir ===
newtype mark<T> = (T,);
/// @generic.template symbol=mark parameters=(out T)
/// @type.symbol symbol=mark source="newtype mark<T> = (T,)" type=mark
/// @definition.newtype symbol=mark source="newtype mark<T> = (T,)" template=(out T) backing=(T,) constructors=[<T>(T) => mark<T>]
/// @type.symbol symbol=mark.T source=T type=T
/// @resolution.name source=T target=mark.T

@mark(1)
/// @generic.instance id=mark<int64> template=mark arguments=(int64)
/// @decorator.node source=@mark(1) owner="const value = 1" expression=mark target=mark type=mark<int64> kind=newtype parameters=(int64) arguments=(provided(1) as int64) newtype=mark backing=(int64,) generic_arguments=(int64) value=mark<int64>(1)
/// @type.node source=mark type=mark
/// @resolution.name source=mark target=mark
/// @type.node source=1 type=1

const value = 1;
/// @type.symbol symbol=value source=value type=1
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=1 type=1
"#,
    );
}

#[test]
fn test_accept_explicit_decorator_generic_arguments() {
    let session = TestSession::single(
        r#"
newtype mark<T> = (T,);

@mark<int32>(1)
const value = 1;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types().with_decorators(),
        r#"
=== annotated ===
newtype mark<out T> = (T,);

@mark<int32>(1)
const value: 1 = 1;

=== dir ===
newtype mark<T> = (T,);
/// @generic.template symbol=mark parameters=(out T)
/// @type.symbol symbol=mark source="newtype mark<T> = (T,)" type=mark
/// @definition.newtype symbol=mark source="newtype mark<T> = (T,)" template=(out T) backing=(T,) constructors=[<T>(T) => mark<T>]
/// @type.symbol symbol=mark.T source=T type=T
/// @resolution.name source=T target=mark.T

@mark<int32>(1)
/// @generic.instance id=mark<int32> template=mark arguments=(int32)
/// @decorator.node source=@mark<int32>(1) owner="const value = 1" expression=mark target=mark type=mark<int32> kind=newtype parameters=(int32) arguments=(provided(1) as int32) newtype=mark backing=(int32,) generic_arguments=(int32) value=mark<int32>(1)
/// @type.node source=mark type=mark
/// @resolution.name source=mark target=mark
/// @type.node source=1 type=1

const value = 1;
/// @type.symbol symbol=value source=value type=1
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=1 type=1
"#,
    );
}
