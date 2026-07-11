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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Receiver = ThisParameterType<(this: { id: string }, value: number) => void>;

const ok: Receiver = { id: "u1" };
ok.id satisfies string;

=== checked ===
type Receiver = ThisParameterType<(this: { id: string }, value: number) => void>;
/// @type.symbol symbol=Receiver source="type Receiver = ThisParameterType<(this: { id: string }, value: number) => void>" type=ThisParameterType<Function<(float64,), void>> reduced={ id: string }
/// @definition.type symbol=Receiver source="type Receiver = ThisParameterType<(this: { id: string }, value: number) => void>" value=ThisParameterType<Function<(float64,), void>> reduced={ id: string }
/// @resolution.name source=ThisParameterType target=types.function.ThisParameterType

const ok: Receiver = { id: "u1" };
/// @type.symbol symbol=ok source=ok type=Receiver reduced={ id: string }
/// @resolution.name source=Receiver target=Receiver

ok.id satisfies string;
/// @resolution.name source=ok target=ok
/// @resolution.member source=ok.id receiver={ id: string } kind=field key=id

/// @generic.instance id="ThisParameterType<Function<(float64,), void>>" template=types.function.ThisParameterType arguments=(Function<(float64,), void>)
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Receiver = ThisParameterType<(value: number) => void>;

const ok: Receiver = { anything: true } as Receiver;

=== checked ===
type Receiver = ThisParameterType<(value: number) => void>;
/// @type.symbol symbol=Receiver source="type Receiver = ThisParameterType<(value: number) => void>" type=ThisParameterType<Function<(float64,), void>> reduced=unknown
/// @definition.type symbol=Receiver source="type Receiver = ThisParameterType<(value: number) => void>" value=ThisParameterType<Function<(float64,), void>> reduced=unknown
/// @resolution.name source=ThisParameterType target=types.function.ThisParameterType

const ok: Receiver = { anything: true };
/// @type.symbol symbol=ok source=ok type=Receiver reduced=unknown
/// @resolution.name source=Receiver target=Receiver

/// @generic.instance id="ThisParameterType<Function<(float64,), void>>" template=types.function.ThisParameterType arguments=(Function<(float64,), void>)
"#,
    );
}

#[test]
fn test_omit_this_parameter_removes_explicit_receiver() {
    let session = TestSession::single(
        r#"
type Fn = OmitThisParameter<(this: { id: string }, value: number) => string>;

const fn: Fn = (value) => `${value}`;
fn(1) satisfies string;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Fn = OmitThisParameter<(this: { id: string }, value: number) => string>;

const fn: Fn = (value: float64): string => `${value}`;
fn(1) satisfies string;

=== checked ===
type Fn = OmitThisParameter<(this: { id: string }, value: number) => string>;
/// @type.symbol symbol=Fn source="type Fn = OmitThisParameter<(this: { id: string }, value: number) => string>" type=OmitThisParameter<Function<(float64,), string>> reduced=Function<(float64,), string>
/// @definition.type symbol=Fn source="type Fn = OmitThisParameter<(this: { id: string }, value: number) => string>" value=OmitThisParameter<Function<(float64,), string>> reduced=Function<(float64,), string>
/// @resolution.name source=OmitThisParameter target=types.function.OmitThisParameter

const fn: Fn = (value) => `${value}`;
/// @type.symbol symbol=fn source=fn type=Fn reduced=Function<(float64,), string>
/// @resolution.name source=Fn target=Fn
/// @type.symbol symbol=symbol6 source="(value) => `${value}`" type=Function<(float64,), string>
/// @type.symbol symbol=symbol6.value source=value type=float64
/// @resolution.name source=value target=symbol6.value

fn(1) satisfies string;
/// @resolution.name source=fn target=fn
/// @resolution.call source=fn(1) parameters=(float64) arguments=(provided(1) as float64) return=string kind=expression

/// @generic.instance id="OmitThisParameter<Function<(float64,), string>>" template=types.function.OmitThisParameter arguments=(Function<(float64,), string>)
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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Fn = OmitThisParameter<(this: { id: string }, value: number) => string>;

declare const fn: Fn;
fn("bad");

=== checked ===
type Fn = OmitThisParameter<(this: { id: string }, value: number) => string>;
/// @type.symbol symbol=Fn source="type Fn = OmitThisParameter<(this: { id: string }, value: number) => string>" type=OmitThisParameter<Function<(float64,), string>> reduced=Function<(float64,), string>
/// @definition.type symbol=Fn source="type Fn = OmitThisParameter<(this: { id: string }, value: number) => string>" value=OmitThisParameter<Function<(float64,), string>> reduced=Function<(float64,), string>
/// @resolution.name source=OmitThisParameter target=types.function.OmitThisParameter

declare const fn: Fn;
/// @type.symbol symbol=fn source=fn type=Fn reduced=Function<(float64,), string>
/// @resolution.name source=Fn target=Fn

fn("bad");
/// @resolution.name source=fn target=fn
/// @resolution.call source="fn(\"bad\")" parameters=(float64) arguments=(provided("bad") as float64) return=string kind=expression

/// @generic.instance id="OmitThisParameter<Function<(float64,), string>>" template=types.function.OmitThisParameter arguments=(Function<(float64,), string>)
"#,
        r#"
/// @diagnostic.error code=EC209 message="argument of type '\"bad\"' is not assignable to parameter of type 'float64'"
/// @diagnostic.label line=5 column=4 span="\"bad\"" line_source="fn(\"bad\");"
/// @diagnostic.related line=5 column=1 span="fn(\"bad\")" line_source="fn(\"bad\");" message="in this call"
"#,
    );
}
