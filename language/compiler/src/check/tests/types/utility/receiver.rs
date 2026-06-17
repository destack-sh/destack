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
/// @type.symbol symbol=Receiver source="type Receiver = ThisParameterType<(this: { id: string }, value: number) => void>" type={ id: string }
/// @definition.type symbol=Receiver source="type Receiver = ThisParameterType<(this: { id: string }, value: number) => void>" value={ id: string }
/// @resolution.name source=ThisParameterType target=types.function.ThisParameterType

const ok: Receiver = { id: "u1" };
/// @type.symbol symbol=ok source=ok type={ id: string }
/// @resolution.name source=Receiver target=Receiver

ok.id satisfies string;
/// @resolution.name source=ok target=ok
/// @resolution.member source=ok.id receiver=Receiver kind=field key=id
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

const ok: Receiver = { anything: true };

=== checked ===
type Receiver = ThisParameterType<(value: number) => void>;
/// @type.symbol symbol=Receiver source="type Receiver = ThisParameterType<(value: number) => void>" type=unknown
/// @definition.type symbol=Receiver source="type Receiver = ThisParameterType<(value: number) => void>" value=unknown
/// @resolution.name source=ThisParameterType target=types.function.ThisParameterType

const ok: Receiver = { anything: true };
/// @type.symbol symbol=ok source=ok type=unknown
/// @resolution.name source=Receiver target=Receiver
"#,
    );
}

#[test]
fn test_omit_this_parameter_removes_explicit_receiver() {
    let session = TestSession::single(
        r#"
type Fn = OmitThisParameter<(this: { id: string }, value: number) => string>;

const fn: Fn = (value) => String(value);
fn(1) satisfies string;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Fn = OmitThisParameter<(this: { id: string }, value: number) => string>;

const fn: Fn = (value: number) => String(value);
fn(1) satisfies string;

=== checked ===
type Fn = OmitThisParameter<(this: { id: string }, value: number) => string>;
/// @type.symbol symbol=Fn source="type Fn = OmitThisParameter<(this: { id: string }, value: number) => string>" type=(number) => string
/// @definition.type symbol=Fn source="type Fn = OmitThisParameter<(this: { id: string }, value: number) => string>" value=(number) => string
/// @resolution.name source=OmitThisParameter target=types.function.OmitThisParameter

const fn: Fn = (value) => String(value);
/// @type.symbol symbol=fn source=fn type=(number) => string
/// @resolution.name source=Fn target=Fn

fn(1) satisfies string;
/// @resolution.name source=fn target=fn
/// @resolution.call source=fn(1) parameters=(number) return=string kind=symbol target=fn
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
/// @type.symbol symbol=Fn source="type Fn = OmitThisParameter<(this: { id: string }, value: number) => string>" type=(number) => string
/// @definition.type symbol=Fn source="type Fn = OmitThisParameter<(this: { id: string }, value: number) => string>" value=(number) => string
/// @resolution.name source=OmitThisParameter target=types.function.OmitThisParameter

declare const fn: Fn;
/// @type.symbol symbol=fn source=fn type=(number) => string
/// @resolution.name source=Fn target=Fn

fn("bad");
/// @resolution.name source=fn target=fn
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '\"bad\"' is not assignable to type 'number'"
/// @diagnostic.label line=5 column=1 source="fn(\"bad\");"
"#,
    );
}
