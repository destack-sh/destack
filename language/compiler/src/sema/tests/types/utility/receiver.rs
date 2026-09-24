use crate::tests::{DirRows, TestSession};

#[test]
fn test_this_parameter_type_extracts_explicit_receiver() {
    let session = TestSession::single(
        r#"
type Receiver = ThisParameterType<(this: { id: string }, value: number) => void>;

const ok: Receiver = { id: "u1" };
ok.id satisfies string;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Receiver = ThisParameterType<(this: { id: string }, value: float64) => void>;

const ok: Receiver = { id: "u1" };
ok.id satisfies string;

=== dir ===
type Receiver = ThisParameterType<(this: { id: string }, value: number) => void>;
/// @type.symbol symbol=Receiver source="type Receiver = ThisParameterType<(this: { id: string }, value: number) => void>" type={ id: string }
/// @definition.type symbol=Receiver source="type Receiver = ThisParameterType<(this: { id: string }, value: number) => void>" value=ThisParameterType<(this: { id: string }, float64) => void>
/// @resolution.name source=ThisParameterType target=ThisParameterType
/// @type.symbol symbol=Receiver.id source="id: string" type=string
/// @type.symbol symbol=Receiver.value source="value: number" type=float64

const ok: Receiver = { id: "u1" };
/// @type.symbol symbol=ok source=ok type=Receiver
/// @resolution.pattern source=ok kind=binding target=ok
/// @resolution.name source=Receiver target=Receiver

ok.id satisfies string;
/// @resolution.name source=ok target=ok
/// @resolution.member source=ok.id receiver=Receiver type=string kind=field target_receiver=Receiver key=id target_type=string
/// @resolution.place source=ok placement="local" lifetime="static" access="immutable"
/// @resolution.access source=ok root=ok
/// @resolution.place source=ok.id placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=ok.id root=ok keys=[id]
"#,
    );
}

#[test]
fn test_this_parameter_type_returns_unknown_without_receiver() {
    let session = TestSession::single(
        r#"
type Receiver = ThisParameterType<(value: number) => void>;

const ok: Receiver = { anything: true };
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Receiver = ThisParameterType<(value: float64) => void>;

const ok: Receiver = { anything: true } as Receiver;

=== dir ===
type Receiver = ThisParameterType<(value: number) => void>;
/// @type.symbol symbol=Receiver source="type Receiver = ThisParameterType<(value: number) => void>" type=unknown
/// @definition.type symbol=Receiver source="type Receiver = ThisParameterType<(value: number) => void>" value=ThisParameterType<(float64) => void>
/// @resolution.name source=ThisParameterType target=ThisParameterType
/// @type.symbol symbol=Receiver.value source="value: number" type=float64

const ok: Receiver = { anything: true };
/// @type.symbol symbol=ok source=ok type=Receiver
/// @resolution.pattern source=ok kind=binding target=ok
/// @resolution.name source=Receiver target=Receiver
"#,
    );
}

#[test]
fn test_omit_this_parameter_removes_explicit_receiver() {
    let session = TestSession::single(
        r#"
type Fn = OmitThisParameter<(this: { id: string }, value: string) => string>;

const fn: Fn = (value) => `${value}`;
fn("one") satisfies string;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Fn = OmitThisParameter<(this: { id: string }, value: string) => string>;

const fn: Fn = (value: string): string => `${value}`;
fn("one") satisfies string;

=== dir ===
type Fn = OmitThisParameter<(this: { id: string }, value: string) => string>;
/// @type.symbol symbol=Fn source="type Fn = OmitThisParameter<(this: { id: string }, value: string) => string>" type=(this: { id: string }, string) => string
/// @definition.type symbol=Fn source="type Fn = OmitThisParameter<(this: { id: string }, value: string) => string>" value=OmitThisParameter<(this: { id: string }, string) => string>
/// @resolution.name source=OmitThisParameter target=OmitThisParameter
/// @type.symbol symbol=Fn.id source="id: string" type=string
/// @type.symbol symbol=Fn.value source="value: string" type=string

const fn: Fn = (value) => `${value}`;
/// @type.symbol symbol=fn source=fn type=Fn
/// @resolution.pattern source=fn kind=binding target=fn
/// @resolution.name source=Fn target=Fn
/// @type.symbol symbol=symbol6 source="(value) => `${value}`" type=Function<(string,), string, "readonly">
/// @type.symbol symbol=symbol6.value source=value type=string
/// @resolution.template source=`${value}` spans=[Display.display(parameters=(), arguments=(), return=^string, regions=("managed" & "local"))] build="stringFromTemplate(parameters=(&'frame readonly Slice<string>, &'frame readonly Slice<string>), arguments=(supplied(0) as &'frame readonly Slice<string>, supplied(1) as &'frame readonly Slice<string>), return=string, regions=(\"frame\", \"frame\"))"
/// @generic.instantiation id="Display.display<string, \"managed\" & \"local\">" template=Display.display arguments=("managed" & "local")
/// @generic.instantiation id="stringFromTemplate<\"frame\", \"frame\">" template=stringFromTemplate arguments=("frame", "frame")
/// @generic.instance id="stringFromTemplate<\"frame\", \"frame\">" template=stringFromTemplate arguments=("frame", "frame")
/// @resolution.name source=value target=symbol6.value
/// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
/// @resolution.access source=value root=symbol6.value

fn("one") satisfies string;
/// @resolution.name source=fn target=fn
/// @resolution.call source="fn(\"one\")" parameters=(string) arguments=(provided("one") as string) return=string kind=expression target=expression
/// @resolution.place source=fn placement="local" lifetime="static" access="immutable"
/// @resolution.access source=fn root=fn
"#,
    );
}

#[test]
fn test_omit_this_parameter_keeps_argument_types() {
    let session = TestSession::single(
        r#"
type Fn = OmitThisParameter<(this: { id: string }, value: number) => string>;

declare const fn: Fn;
fn("bad");
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Fn = OmitThisParameter<(this: { id: string }, value: float64) => string>;

declare const fn: Fn;
fn("bad");

=== dir ===
type Fn = OmitThisParameter<(this: { id: string }, value: number) => string>;
/// @type.symbol symbol=Fn source="type Fn = OmitThisParameter<(this: { id: string }, value: number) => string>" type=(this: { id: string }, float64) => string
/// @definition.type symbol=Fn source="type Fn = OmitThisParameter<(this: { id: string }, value: number) => string>" value=OmitThisParameter<(this: { id: string }, float64) => string>
/// @resolution.name source=OmitThisParameter target=OmitThisParameter
/// @type.symbol symbol=Fn.id source="id: string" type=string
/// @type.symbol symbol=Fn.value source="value: number" type=float64

declare const fn: Fn;
/// @type.symbol symbol=fn source=fn type=Fn
/// @resolution.pattern source=fn kind=binding target=fn
/// @resolution.name source=Fn target=Fn

fn("bad");
/// @resolution.name source=fn target=fn
/// @resolution.call source="fn(\"bad\")" parameters=(float64) arguments=(provided("bad") as float64) return=string kind=expression target=expression
/// @resolution.place source=fn placement="local" lifetime="static" access="immutable"
/// @resolution.access source=fn root=fn
"#,
        r#"
/// @diagnostic.error id=argument-not-assignable message="argument of type '\"bad\"' is not assignable to parameter of type 'float64'"
/// @diagnostic.label line=5 column=4 span="\"bad\"" line_source="fn(\"bad\");"
/// @diagnostic.related line=5 column=1 span="fn(\"bad\")" line_source="fn(\"bad\");" message="in this call"
"#,
    );
}
