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

const ok: { id: string } = { id: "u1" };
ok.id satisfies string;

=== dir ===
type Receiver = ThisParameterType<(this: { id: string }, value: number) => void>;
/// @type.symbol symbol=Receiver source="type Receiver = ThisParameterType<(this: { id: string }, value: number) => void>" type={ id: string }
/// @definition.type symbol=Receiver source="type Receiver = ThisParameterType<(this: { id: string }, value: number) => void>" value={ id: string }
/// @resolution.name source=ThisParameterType target=ThisParameterType
/// @type.symbol symbol=Receiver.this source="this: { id: string }" type={ id: string }
/// @type.symbol symbol=Receiver.id source="id: string" type=string
/// @type.symbol symbol=Receiver.value source="value: number" type=float64

const ok: Receiver = { id: "u1" };
/// @type.symbol symbol=ok source=ok type={ id: string }
/// @resolution.pattern source=ok kind=binding target=ok
/// @resolution.name source=Receiver target=Receiver

ok.id satisfies string;
/// @resolution.name source=ok target=ok
/// @resolution.member source=ok.id receiver={ id: string } type=string kind=field target_receiver={ id: string } key=id target_type=string
/// @resolution.place source=ok placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=ok root=ok
/// @resolution.place source=ok.id placement="local" lifetime="static" access="exclusive"
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

const ok: unknown = { anything: true } as unknown;

=== dir ===
type Receiver = ThisParameterType<(value: number) => void>;
/// @type.symbol symbol=Receiver source="type Receiver = ThisParameterType<(value: number) => void>" type=unknown
/// @definition.type symbol=Receiver source="type Receiver = ThisParameterType<(value: number) => void>" value=unknown
/// @resolution.name source=ThisParameterType target=ThisParameterType
/// @type.symbol symbol=Receiver.value source="value: number" type=float64

const ok: Receiver = { anything: true };
/// @type.symbol symbol=ok source=ok type=unknown
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

const fn: (arg0: string) => string = (value: string): string => `${value}`;
fn("one") satisfies string;

=== dir ===
type Fn = OmitThisParameter<(this: { id: string }, value: string) => string>;
/// @type.symbol symbol=Fn source="type Fn = OmitThisParameter<(this: { id: string }, value: string) => string>" type=Function<(string,), string>
/// @definition.type symbol=Fn source="type Fn = OmitThisParameter<(this: { id: string }, value: string) => string>" value=Function<(string,), string>
/// @resolution.name source=OmitThisParameter target=OmitThisParameter
/// @type.symbol symbol=Fn.this source="this: { id: string }" type={ id: string }
/// @type.symbol symbol=Fn.id source="id: string" type=string
/// @type.symbol symbol=Fn.value source="value: string" type=string

const fn: Fn = (value) => `${value}`;
/// @type.symbol symbol=fn source=fn type=Function<(string,), string>
/// @resolution.pattern source=fn kind=binding target=fn
/// @resolution.name source=Fn target=Fn
/// @type.symbol symbol=symbol6 source="(value) => `${value}`" type=Function<(string,), string>
/// @type.symbol symbol=symbol6.value source=value type=string
/// @resolution.name source=value target=symbol6.value
/// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
/// @resolution.access source=value root=symbol6.value

fn("one") satisfies string;
/// @resolution.name source=fn target=fn
/// @resolution.call source="fn(\"one\")" parameters=(string) arguments=(provided("one") as string) return=string kind=expression target=expression
/// @resolution.place source=fn placement="local" lifetime="static" access="exclusive"
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

declare const fn: (arg0: float64) => string;
fn("bad");

=== dir ===
type Fn = OmitThisParameter<(this: { id: string }, value: number) => string>;
/// @type.symbol symbol=Fn source="type Fn = OmitThisParameter<(this: { id: string }, value: number) => string>" type=Function<(float64,), string>
/// @definition.type symbol=Fn source="type Fn = OmitThisParameter<(this: { id: string }, value: number) => string>" value=Function<(float64,), string>
/// @resolution.name source=OmitThisParameter target=OmitThisParameter
/// @type.symbol symbol=Fn.this source="this: { id: string }" type={ id: string }
/// @type.symbol symbol=Fn.id source="id: string" type=string
/// @type.symbol symbol=Fn.value source="value: number" type=float64

declare const fn: Fn;
/// @type.symbol symbol=fn source=fn type=Function<(float64,), string>
/// @resolution.pattern source=fn kind=binding target=fn
/// @resolution.name source=Fn target=Fn

fn("bad");
/// @resolution.name source=fn target=fn
/// @resolution.call source="fn(\"bad\")" parameters=(float64) arguments=(provided("bad") as float64) return=string kind=expression target=expression
/// @resolution.place source=fn placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=fn root=fn
"#,
        r#"
/// @diagnostic.error id=argument-not-assignable message="argument of type '\"bad\"' is not assignable to parameter of type 'float64'"
/// @diagnostic.label line=5 column=4 span="\"bad\"" line_source="fn(\"bad\");"
/// @diagnostic.related line=5 column=1 span="fn(\"bad\")" line_source="fn(\"bad\");" message="in this call"
"#,
    );
}
