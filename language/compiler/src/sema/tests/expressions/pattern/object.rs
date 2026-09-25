use crate::tests::{DirRows, TestSession};

/// Bind each field of an object pattern to its own name.
#[test]
fn test_object_pattern_binds_fields() {
    let session = TestSession::single(
        r#"
declare const point: { x: int32; y: string };

let { x, y } = point;

x satisfies int32;
y satisfies string;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const point: { x: int32; y: string };

let { x, y } = point;

x satisfies int32;
y satisfies string;

=== dir ===
declare const point: { x: int32; y: string };
/// @type.symbol symbol=point source=point type={ x: int32; y: string }
/// @resolution.pattern source=point kind=binding target=point
/// @type.symbol symbol=x#1 source="x: int32" type=int32
/// @type.symbol symbol=y#1 source="y: string" type=string

let { x, y } = point;
/// @resolution.pattern source={ x, y } kind=object fields={ x, y }
/// @type.symbol symbol=x#2 source=x type=int32
/// @type.symbol symbol=y#2 source=y type=string
/// @type.node source=point type={ x: int32; y: string }
/// @resolution.name source=point target=point
/// @resolution.access source=point root=point

x satisfies int32;
/// @type.node source="x satisfies int32" type=int32
/// @type.node source=x type=int32
/// @resolution.name source=x target=x#2
/// @resolution.place source=x placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=x root=x#2

y satisfies string;
/// @type.node source="y satisfies string" type=string
/// @type.node source=y type=string
/// @resolution.name source=y target=y#2
/// @resolution.place source=y placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=y root=y#2
"#,
    );
}

/// Check the value of an object pattern against its written annotation.
#[test]
fn test_object_pattern_checks_annotated_value() {
    let session = TestSession::single(
        r#"
declare const source: { x: string };

let { x }: { x: int32 } = source;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const source: { x: string };

let { x }: { x: int32 } = source;

=== dir ===
declare const source: { x: string };
/// @type.symbol symbol=source source=source type={ x: string }
/// @resolution.pattern source=source kind=binding target=source
/// @type.symbol symbol=x#1 source="x: string" type=string

let { x }: { x: int32 } = source;
/// @resolution.pattern source={ x } kind=object fields={ x }
/// @type.symbol symbol=x#3 source=x type=int32
/// @type.symbol symbol=x#2 source="x: int32" type=int32
/// @type.node source=source type={ x: string }
/// @resolution.name source=source target=source
/// @resolution.place source=source placement="local" lifetime="static" access="immutable"
/// @resolution.access source=source root=source
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '{ x: string }' is not assignable to type '{ x: int32 }'"
/// @diagnostic.label line=4 column=27 span="source" line_source="let { x }: { x: int32 } = source;"
/// @diagnostic.related line=4 column=12 span="{ x: int32 }" line_source="let { x }: { x: int32 } = source;" message="expected due to this annotation"
/// @diagnostic.note message="the mismatch is in field 'x': expected 'int32', found 'string'"
"#,
    );
}

/// Reject an object pattern over a primitive value.
#[test]
fn test_object_pattern_rejects_primitive_value() {
    let session = TestSession::single(
        r#"
let { value } = 1;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let { value } = 1;

=== dir ===
let { value } = 1;
/// @resolution.rejected source={ value }
/// @type.symbol symbol=value source=value type=<error>
/// @type.node source=1 type=1
"#,
        r#"
/// @diagnostic.error id=pattern-source-not-object-shaped message="type '1' cannot be destructured as an object pattern"
/// @diagnostic.label line=2 column=5 span="{ value }" line_source="let { value } = 1;"
"#,
    );
}

/// Narrow a union scrutinee through a nested object pattern discriminant.
#[test]
fn test_nested_object_pattern_narrows_by_inner_discriminant() {
    let session = TestSession::single(
        r#"
type State =
    | { inner: { kind: "a"; value: int32 } }
    | { inner: { kind: "b"; flag: boolean } };

declare const state: State;

const result = match (state) {
    { inner: { kind: "a", value } } => value
    { inner: { kind: "b", flag } } => 0
};
"#,
    );

    session.assert_dir("main.tspp", DirRows::checked(), r#"
=== annotated ===
type State = { inner: { kind: "a"; value: int32 } } | { inner: { kind: "b"; flag: boolean } };

declare const state: State;

const result: int32 = match (state) {
    {
        inner: { kind: "a", value },
    } => value
    {
        inner: { kind: "b", flag },
    } => 0
};

=== dir ===
type State =
/// @type.symbol symbol=State type={ inner: { kind: "a"; value: int32 } } | { inner: { kind: "b"; flag: boolean } }
/// @definition.type symbol=State value={ inner: { kind: "a"; value: int32 } } | { inner: { kind: "b"; flag: boolean } }

    | { inner: { kind: "a"; value: int32 } }
    /// @type.symbol symbol=State.inner#1 source="inner: { kind: \"a\"; value: int32 }" type={ kind: "a"; value: int32 }
    /// @type.symbol symbol=State.kind#1 source="kind: \"a\"" type="a"
    /// @type.symbol symbol=State.value source="value: int32" type=int32

    | { inner: { kind: "b"; flag: boolean } };
    /// @type.symbol symbol=State.inner#2 source="inner: { kind: \"b\"; flag: boolean }" type={ kind: "b"; flag: boolean }
    /// @type.symbol symbol=State.kind#2 source="kind: \"b\"" type="b"
    /// @type.symbol symbol=State.flag source="flag: boolean" type=boolean

declare const state: State;
/// @type.symbol symbol=state source=state type=State
/// @resolution.pattern source=state kind=binding target=state
/// @resolution.name source=State target=State

const result = match (state) {
/// @type.symbol symbol=result source=result type=int32
/// @resolution.pattern source=result kind=binding target=result
/// @resolution.coverage exhaustive=true disjoint=true
/// @resolution.name source=state target=state
/// @resolution.place source=state placement="local" lifetime="static" access="immutable"
/// @resolution.access source=state root=state

    { inner: { kind: "a", value } } => value
    /// @resolution.pattern source={ inner: { kind: "a", value } } kind=object adjustments=(union.payload({ inner: { kind: "a"; value: int32 } } | { inner: { kind: "b"; flag: boolean } }, { inner: { kind: "a"; value: int32 } }, { inner: { kind: "a"; value: int32 } })) fields={ inner: pattern }
    /// @resolution.pattern source={ kind: "a", value } kind=object fields={ kind: "a", value }
    /// @resolution.pattern source="\"a\"" kind=literal value="a"
    /// @type.symbol symbol=value source=value type=int32
    /// @resolution.name source=value target=value
    /// @resolution.place source=value placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=value root=value

    { inner: { kind: "b", flag } } => 0
    /// @resolution.pattern source={ inner: { kind: "b", flag } } kind=object adjustments=(union.payload({ inner: { kind: "a"; value: int32 } } | { inner: { kind: "b"; flag: boolean } }, { inner: { kind: "b"; flag: boolean } }, { inner: { kind: "b"; flag: boolean } })) fields={ inner: pattern }
    /// @resolution.pattern source={ kind: "b", flag } kind=object fields={ kind: "b", flag }
    /// @resolution.pattern source="\"b\"" kind=literal value="b"
    /// @type.symbol symbol=flag source=flag type=boolean

};
"#);
}

/// Narrow a struct union by a literal field in a flat object pattern.
#[test]
fn test_object_pattern_narrows_struct_union_by_literal_field() {
    let session = TestSession::single(
        r#"
struct Pending {
    kind: "pending";
    waiting: int32;
}

struct Ready {
    kind: "ready";
    value: int32;
}

type State = Pending | Ready;

declare const state: State;

const result = match (state) {
    { kind: "pending", waiting } => waiting
    { kind: "ready", value } => value
};
"#,
    );

    session.assert_dir("main.tspp", DirRows::checked().with_reference_types(), r#"
=== annotated ===
struct Pending {
    kind: "pending";
    waiting: int32;
}

struct Ready {
    kind: "ready";
    value: int32;
}

type State = Pending | Ready;

declare const state: State;

const result: int32 = match (state) {
    { kind: "pending", waiting } => waiting
    { kind: "ready", value } => value
};

=== dir ===
struct Pending {
/// @type.symbol symbol=Pending type=Pending
/// @definition.struct symbol=Pending
/// @definition.field symbol=Pending.kind source="kind: \"pending\"" key=kind type="pending"
/// @definition.field symbol=Pending.waiting source="waiting: int32" key=waiting type=int32

    kind: "pending";
    /// @type.symbol symbol=Pending.kind source="kind: \"pending\"" type="pending"

    waiting: int32;
    /// @type.symbol symbol=Pending.waiting source="waiting: int32" type=int32

}

struct Ready {
/// @type.symbol symbol=Ready type=Ready
/// @definition.struct symbol=Ready
/// @definition.field symbol=Ready.kind source="kind: \"ready\"" key=kind type="ready"
/// @definition.field symbol=Ready.value source="value: int32" key=value type=int32

    kind: "ready";
    /// @type.symbol symbol=Ready.kind source="kind: \"ready\"" type="ready"

    value: int32;
    /// @type.symbol symbol=Ready.value source="value: int32" type=int32

}

type State = Pending | Ready;
/// @type.symbol symbol=State source="type State = Pending | Ready" type=Pending | Ready
/// @definition.type symbol=State source="type State = Pending | Ready" value=Pending | Ready
/// @resolution.name source=Pending target=Pending
/// @resolution.name source=Ready target=Ready

declare const state: State;
/// @type.symbol symbol=state source=state type=State
/// @resolution.pattern source=state kind=binding target=state
/// @resolution.name source=State target=State

const result = match (state) {
/// @type.symbol symbol=result source=result type=int32
/// @resolution.pattern source=result kind=binding target=result
/// @type.node type=int32
/// @resolution.coverage exhaustive=true disjoint=false
/// @type.node source=state type=State
/// @resolution.name source=state target=state
/// @resolution.place source=state placement="local" lifetime="static" access="immutable"
/// @resolution.access source=state root=state

    { kind: "pending", waiting } => waiting
    /// @resolution.pattern source={ kind: "pending", waiting } kind=object fields={ Pending.kind: "pending", Pending.waiting }
    /// @type.node source="\"pending\"" type="pending"
    /// @resolution.pattern source="\"pending\"" kind=literal value="pending"
    /// @type.symbol symbol=waiting source=waiting type=int32
    /// @type.node source=waiting type=int32
    /// @resolution.name source=waiting target=waiting
    /// @resolution.place source=waiting placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=waiting root=waiting

    { kind: "ready", value } => value
    /// @resolution.pattern source={ kind: "ready", value } kind=object fields={ Ready.kind: "ready", Ready.value }
    /// @type.node source="\"ready\"" type="ready"
    /// @resolution.pattern source="\"ready\"" kind=literal value="ready"
    /// @type.symbol symbol=value source=value type=int32
    /// @type.node source=value type=int32
    /// @resolution.name source=value target=value
    /// @resolution.place source=value placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=value root=value

};
"#);
}

/// Narrow a struct union by an integer literal field in an object pattern.
#[test]
fn test_object_pattern_narrows_by_integer_discriminant() {
    let session = TestSession::single(
        r#"
struct Header {
    version: 1;
    length: int32;
}

struct Trailer {
    version: 2;
    checksum: int32;
}

type Frame = Header | Trailer;

declare const frame: Frame;

const result = match (frame) {
    { version: 1, length } => length
    { version: 2, checksum } => checksum
};
"#,
    );

    session.assert_dir("main.tspp", DirRows::checked().with_reference_types(), r#"
=== annotated ===
struct Header {
    version: 1;
    length: int32;
}

struct Trailer {
    version: 2;
    checksum: int32;
}

type Frame = Header | Trailer;

declare const frame: Frame;

const result: int32 = match (frame) {
    { version: 1, length } => length
    { version: 2, checksum } => checksum
};

=== dir ===
struct Header {
/// @type.symbol symbol=Header type=Header
/// @definition.struct symbol=Header
/// @definition.field symbol=Header.length source="length: int32" key=length type=int32
/// @definition.field symbol=Header.version source="version: 1" key=version type=1

    version: 1;
    /// @type.symbol symbol=Header.version source="version: 1" type=1

    length: int32;
    /// @type.symbol symbol=Header.length source="length: int32" type=int32

}

struct Trailer {
/// @type.symbol symbol=Trailer type=Trailer
/// @definition.struct symbol=Trailer
/// @definition.field symbol=Trailer.checksum source="checksum: int32" key=checksum type=int32
/// @definition.field symbol=Trailer.version source="version: 2" key=version type=2

    version: 2;
    /// @type.symbol symbol=Trailer.version source="version: 2" type=2

    checksum: int32;
    /// @type.symbol symbol=Trailer.checksum source="checksum: int32" type=int32

}

type Frame = Header | Trailer;
/// @type.symbol symbol=Frame source="type Frame = Header | Trailer" type=Header | Trailer
/// @definition.type symbol=Frame source="type Frame = Header | Trailer" value=Header | Trailer
/// @resolution.name source=Header target=Header
/// @resolution.name source=Trailer target=Trailer

declare const frame: Frame;
/// @type.symbol symbol=frame source=frame type=Frame
/// @resolution.pattern source=frame kind=binding target=frame
/// @resolution.name source=Frame target=Frame

const result = match (frame) {
/// @type.symbol symbol=result source=result type=int32
/// @resolution.pattern source=result kind=binding target=result
/// @type.node type=int32
/// @resolution.coverage exhaustive=true disjoint=false
/// @type.node source=frame type=Frame
/// @resolution.name source=frame target=frame
/// @resolution.place source=frame placement="local" lifetime="static" access="immutable"
/// @resolution.access source=frame root=frame

    { version: 1, length } => length
    /// @resolution.pattern source={ version: 1, length } kind=object fields={ Header.version: 1, Header.length }
    /// @type.node source=1 type=1
    /// @resolution.pattern source=1 kind=literal value=1
    /// @type.symbol symbol=length source=length type=int32
    /// @type.node source=length type=int32
    /// @resolution.name source=length target=length
    /// @resolution.place source=length placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=length root=length

    { version: 2, checksum } => checksum
    /// @resolution.pattern source={ version: 2, checksum } kind=object fields={ Trailer.version: 2, Trailer.checksum }
    /// @type.node source=2 type=2
    /// @resolution.pattern source=2 kind=literal value=2
    /// @type.symbol symbol=checksum source=checksum type=int32
    /// @type.node source=checksum type=int32
    /// @resolution.name source=checksum target=checksum
    /// @resolution.place source=checksum placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=checksum root=checksum

};
"#);
}

/// Narrow a newtype union through a nested object pattern discriminant.
#[test]
fn test_nested_object_pattern_narrows_newtype_union() {
    let session = TestSession::single(
        r#"
struct Alpha {
    kind: "a" = "a";
    value: int32;
}

struct Beta {
    kind: "b" = "b";
    flag: boolean;
}

struct Opened {
    inner: Alpha;
}

struct Closed {
    inner: Beta;
}

newtype Envelope = Opened | Closed;

declare const envelope: Envelope;

const result = match (envelope) {
    { inner: { kind: "a", value } } => value
    { inner: { kind: "b", flag } } => 0
};
"#,
    );

    session.assert_dir("main.tspp", DirRows::checked().with_reference_types(), r#"
=== annotated ===
struct Alpha {
    kind: "a" = "a";
    value: int32;
}

struct Beta {
    kind: "b" = "b";
    flag: boolean;
}

struct Opened {
    inner: Alpha;
}

struct Closed {
    inner: Beta;
}

newtype Envelope = Opened | Closed;

declare const envelope: Envelope;

const result: int32 = match (envelope) {
    {
        inner: { kind: "a", value },
    } => value
    {
        inner: { kind: "b", flag },
    } => 0
};

=== dir ===
struct Alpha {
/// @type.symbol symbol=Alpha type=Alpha
/// @definition.struct symbol=Alpha
/// @definition.field symbol=Alpha.kind source="kind: \"a\" = \"a\"" key=kind type="a"
/// @definition.field symbol=Alpha.value source="value: int32" key=value type=int32

    kind: "a" = "a";
    /// @type.symbol symbol=Alpha.kind source="kind: \"a\" = \"a\"" type="a"
    /// @type.node source="\"a\"" type="a"

    value: int32;
    /// @type.symbol symbol=Alpha.value source="value: int32" type=int32

}

struct Beta {
/// @type.symbol symbol=Beta type=Beta
/// @definition.struct symbol=Beta
/// @definition.field symbol=Beta.flag source="flag: boolean" key=flag type=boolean
/// @definition.field symbol=Beta.kind source="kind: \"b\" = \"b\"" key=kind type="b"

    kind: "b" = "b";
    /// @type.symbol symbol=Beta.kind source="kind: \"b\" = \"b\"" type="b"
    /// @type.node source="\"b\"" type="b"

    flag: boolean;
    /// @type.symbol symbol=Beta.flag source="flag: boolean" type=boolean

}

struct Opened {
/// @type.symbol symbol=Opened type=Opened
/// @definition.struct symbol=Opened
/// @definition.field symbol=Opened.inner source="inner: Alpha" key=inner type=Alpha

    inner: Alpha;
    /// @type.symbol symbol=Opened.inner source="inner: Alpha" type=Alpha
    /// @resolution.name source=Alpha target=Alpha

}

struct Closed {
/// @type.symbol symbol=Closed type=Closed
/// @definition.struct symbol=Closed
/// @definition.field symbol=Closed.inner source="inner: Beta" key=inner type=Beta

    inner: Beta;
    /// @type.symbol symbol=Closed.inner source="inner: Beta" type=Beta
    /// @resolution.name source=Beta target=Beta

}

newtype Envelope = Opened | Closed;
/// @type.symbol symbol=Envelope source="newtype Envelope = Opened | Closed" type=Envelope
/// @definition.newtype symbol=Envelope source="newtype Envelope = Opened | Closed" backing=Opened | Closed constructors=[(Opened) => Envelope, (Closed) => Envelope, (Opened | Closed) => Envelope]
/// @resolution.name source=Opened target=Opened
/// @resolution.name source=Closed target=Closed

declare const envelope: Envelope;
/// @type.symbol symbol=envelope source=envelope type=Envelope
/// @resolution.pattern source=envelope kind=binding target=envelope
/// @resolution.name source=Envelope target=Envelope

const result = match (envelope) {
/// @type.symbol symbol=result source=result type=int32
/// @resolution.pattern source=result kind=binding target=result
/// @type.node type=int32
/// @resolution.coverage exhaustive=true disjoint=true
/// @type.node source=envelope type=Envelope
/// @resolution.name source=envelope target=envelope
/// @resolution.place source=envelope placement="local" lifetime="static" access="immutable"
/// @resolution.access source=envelope root=envelope

    { inner: { kind: "a", value } } => value
    /// @resolution.pattern source={ inner: { kind: "a", value } } kind=object adjustments=(newtype.payload(Envelope, Opened | Closed), union.payload(Opened | Closed, Opened, Opened)) fields={ Opened.inner: pattern }
    /// @resolution.pattern source={ kind: "a", value } kind=object fields={ Alpha.kind: "a", Alpha.value }
    /// @type.node source="\"a\"" type="a"
    /// @resolution.pattern source="\"a\"" kind=literal value="a"
    /// @type.symbol symbol=value source=value type=int32
    /// @type.node source=value type=int32
    /// @resolution.name source=value target=value
    /// @resolution.place source=value placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=value root=value

    { inner: { kind: "b", flag } } => 0
    /// @resolution.pattern source={ inner: { kind: "b", flag } } kind=object adjustments=(newtype.payload(Envelope, Opened | Closed), union.payload(Opened | Closed, Closed, Closed)) fields={ Closed.inner: pattern }
    /// @resolution.pattern source={ kind: "b", flag } kind=object fields={ Beta.kind: "b", Beta.flag }
    /// @type.node source="\"b\"" type="b"
    /// @resolution.pattern source="\"b\"" kind=literal value="b"
    /// @type.symbol symbol=flag source=flag type=boolean
    /// @type.node source=0 type=0

};
"#);
}

/// Narrow a borrowed struct union by a literal field in an object pattern.
#[test]
fn test_object_pattern_narrows_borrowed_struct_union() {
    let session = TestSession::single(
        r#"
struct Pending {
    kind: "pending";
    waiting: int32;
}

struct Ready {
    kind: "ready";
    value: int32;
}

type State = Pending | Ready;

declare const state: &readonly State;

const result = match (state) {
    { kind: "pending", waiting } => waiting
    { kind: "ready", value } => value
};
"#,
    );

    session.assert_dir("main.tspp", DirRows::checked().with_reference_types(), r#"
=== annotated ===
struct Pending {
    kind: "pending";
    waiting: int32;
}

struct Ready {
    kind: "ready";
    value: int32;
}

type State = Pending | Ready;

declare const state: &'static readonly State;

const result: int32 = match (state) {
    { kind: "pending", waiting } => waiting
    { kind: "ready", value } => value
};

=== dir ===
struct Pending {
/// @type.symbol symbol=Pending type=Pending
/// @definition.struct symbol=Pending
/// @definition.field symbol=Pending.kind source="kind: \"pending\"" key=kind type="pending"
/// @definition.field symbol=Pending.waiting source="waiting: int32" key=waiting type=int32

    kind: "pending";
    /// @type.symbol symbol=Pending.kind source="kind: \"pending\"" type="pending"

    waiting: int32;
    /// @type.symbol symbol=Pending.waiting source="waiting: int32" type=int32

}

struct Ready {
/// @type.symbol symbol=Ready type=Ready
/// @definition.struct symbol=Ready
/// @definition.field symbol=Ready.kind source="kind: \"ready\"" key=kind type="ready"
/// @definition.field symbol=Ready.value source="value: int32" key=value type=int32

    kind: "ready";
    /// @type.symbol symbol=Ready.kind source="kind: \"ready\"" type="ready"

    value: int32;
    /// @type.symbol symbol=Ready.value source="value: int32" type=int32

}

type State = Pending | Ready;
/// @type.symbol symbol=State source="type State = Pending | Ready" type=Pending | Ready
/// @definition.type symbol=State source="type State = Pending | Ready" value=Pending | Ready
/// @resolution.name source=Pending target=Pending
/// @resolution.name source=Ready target=Ready

declare const state: &readonly State;
/// @type.symbol symbol=state source=state type=&'static readonly State
/// @resolution.pattern source=state kind=binding target=state
/// @resolution.name source=State target=State

const result = match (state) {
/// @type.symbol symbol=result source=result type=int32
/// @resolution.pattern source=result kind=binding target=result
/// @type.node type=int32
/// @resolution.coverage exhaustive=true disjoint=true
/// @type.node source=state type=&'static readonly State
/// @resolution.name source=state target=state
/// @resolution.place source=state placement="local" lifetime="static" access="immutable"
/// @resolution.access source=state root=state

    { kind: "pending", waiting } => waiting
    /// @resolution.pattern source={ kind: "pending", waiting } kind=object adjustments=(union.payload(Pending | Ready, Pending, &'static readonly Pending)) fields={ Pending.kind: "pending", Pending.waiting }
    /// @type.node source="\"pending\"" type="pending"
    /// @resolution.pattern source="\"pending\"" kind=literal value="pending"
    /// @type.symbol symbol=waiting source=waiting type=int32
    /// @type.node source=waiting type=int32
    /// @resolution.name source=waiting target=waiting
    /// @resolution.place source=waiting placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=waiting root=waiting

    { kind: "ready", value } => value
    /// @resolution.pattern source={ kind: "ready", value } kind=object adjustments=(union.payload(Pending | Ready, Ready, &'static readonly Ready)) fields={ Ready.kind: "ready", Ready.value }
    /// @type.node source="\"ready\"" type="ready"
    /// @resolution.pattern source="\"ready\"" kind=literal value="ready"
    /// @type.symbol symbol=value source=value type=int32
    /// @type.node source=value type=int32
    /// @resolution.name source=value target=value
    /// @resolution.place source=value placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=value root=value

};
"#);
}
