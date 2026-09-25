use crate::tests::{DirRows, TestSession};

/// Parameters extracts the argument tuple of a signature.
#[test]
fn test_parameters_extracts_argument_tuple() {
    let session = TestSession::single(
        r#"
type Args = Parameters<(name: string, count: number) => boolean>;

const ok: Args = ("Ada", 1);
ok satisfies (string, number);
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Args = Parameters<(name: string, count: float64) => boolean>;

const ok: Args = ("Ada", 1);
ok satisfies (string, number);

=== dir ===
type Args = Parameters<(name: string, count: number) => boolean>;
/// @type.symbol symbol=Args source="type Args = Parameters<(name: string, count: number) => boolean>" type=(string, float64)
/// @definition.type symbol=Args source="type Args = Parameters<(name: string, count: number) => boolean>" value=Parameters<(string, float64) => boolean>
/// @resolution.name source=Parameters target=Parameters
/// @type.symbol symbol=Args.name source="name: string" type=string
/// @type.symbol symbol=Args.count source="count: number" type=float64

const ok: Args = ("Ada", 1);
/// @type.symbol symbol=ok source=ok type=Args
/// @resolution.pattern source=ok kind=binding target=ok
/// @resolution.name source=Args target=Args

ok satisfies (string, number);
/// @resolution.name source=ok target=ok
/// @resolution.place source=ok placement="local" lifetime="static" access="immutable"
/// @resolution.access source=ok root=ok
"#,
    );
}

/// Parameters keeps the optional positions of a signature.
#[test]
fn test_parameters_preserves_optional_parameters() {
    let session = TestSession::single(
        r#"
type Args = Parameters<(name: string, count?: number) => boolean>;

const short: Args = ("Ada",);
const full: Args = ("Ada", 1);
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Args = Parameters<(name: string, count?: float64) => boolean>;

const short: Args = ("Ada",) as Args;
const full: Args = ("Ada", 1 as float64 | undefined) as Args;

=== dir ===
type Args = Parameters<(name: string, count?: number) => boolean>;
/// @type.symbol symbol=Args source="type Args = Parameters<(name: string, count?: number) => boolean>" type=(string, float64 | undefined?)
/// @definition.type symbol=Args source="type Args = Parameters<(name: string, count?: number) => boolean>" value=Parameters<(string, float64 | undefined?) => boolean>
/// @resolution.name source=Parameters target=Parameters
/// @type.symbol symbol=Args.name source="name: string" type=string
/// @type.symbol symbol=Args.count source="count?: number" type=float64 | undefined

const short: Args = ("Ada",);
/// @type.symbol symbol=short source=short type=Args
/// @resolution.pattern source=short kind=binding target=short
/// @resolution.name source=Args target=Args

const full: Args = ("Ada", 1);
/// @type.symbol symbol=full source=full type=Args
/// @resolution.pattern source=full kind=binding target=full
/// @resolution.name source=Args target=Args
"#,
    );
}

/// Parameters keeps the rest position of a signature.
#[test]
fn test_parameters_preserves_rest_parameters() {
    let session = TestSession::single(
        r#"
type Args = Parameters<(name: string, ...flags: boolean[]) => void>;

const ok: Args = ("Ada", true, false);
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Args = Parameters<(name: string, ...flags: boolean[]) => void>;

const ok: Args = ("Ada", true, false) as Args;

=== dir ===
type Args = Parameters<(name: string, ...flags: boolean[]) => void>;
/// @type.symbol symbol=Args source="type Args = Parameters<(name: string, ...flags: boolean[]) => void>" type=(string, ...boolean[])
/// @generic.instance id=Array<boolean> template=Array arguments=(boolean)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<boolean>> template=sliceAssumeInit arguments=(MaybeUninit<boolean>)
/// @generic.instance id=sliceUninit<MaybeUninit<boolean>> template=sliceUninit arguments=(MaybeUninit<boolean>)
/// @definition.type symbol=Args source="type Args = Parameters<(name: string, ...flags: boolean[]) => void>" value=Parameters<(string, ...boolean[]) => void>
/// @resolution.name source=Parameters target=Parameters
/// @type.symbol symbol=Args.name source="name: string" type=string
/// @type.symbol symbol=Args.flags source="...flags: boolean[]" type=boolean[]

const ok: Args = ("Ada", true, false);
/// @type.symbol symbol=ok source=ok type=Args
/// @resolution.pattern source=ok kind=binding target=ok
/// @resolution.name source=Args target=Args
"#,
    );
}

/// An argument of another type reports a diagnostic against Parameters.
#[test]
fn test_parameters_rejects_wrong_argument_types() {
    let session = TestSession::single(
        r#"
type Args = Parameters<(name: string, count: number) => boolean>;

const bad: Args = ("Ada", "one");
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Args = Parameters<(name: string, count: float64) => boolean>;

const bad: Args = ("Ada", "one");

=== dir ===
type Args = Parameters<(name: string, count: number) => boolean>;
/// @type.symbol symbol=Args source="type Args = Parameters<(name: string, count: number) => boolean>" type=(string, float64)
/// @definition.type symbol=Args source="type Args = Parameters<(name: string, count: number) => boolean>" value=Parameters<(string, float64) => boolean>
/// @resolution.name source=Parameters target=Parameters
/// @type.symbol symbol=Args.name source="name: string" type=string
/// @type.symbol symbol=Args.count source="count: number" type=float64

const bad: Args = ("Ada", "one");
/// @type.symbol symbol=bad source=bad type=Args
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Args target=Args
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"one\"' is not assignable to type 'float64'"
/// @diagnostic.label line=4 column=27 span="\"one\"" line_source="const bad: Args = (\"Ada\", \"one\");"
/// @diagnostic.related line=4 column=12 span="Args" line_source="const bad: Args = (\"Ada\", \"one\");" message="expected due to this annotation"
/// @diagnostic.note message="the mismatch is in element 1"
"#,
    );
}

/// Reduce typeof over an overload group to the signature intersection.
#[test]
fn test_typeof_over_an_overload_group_reduces_to_the_intersection() {
    let session = TestSession::single(
        r#"
function parse(value: int32): int32 {
    value
}

function parse(value: string): string {
    value
}

type Parser = typeof parse;

declare const parser: Parser;

parser satisfies ((value: int32) => int32) & ((value: string) => string);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
function parse(value: int32): int32 {
    value
}

function parse(value: string): string {
    value
}

type Parser = typeof parse;

declare const parser: Parser;

parser satisfies ((value: int32) => int32) & ((value: string) => string);

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

type Parser = typeof parse;
/// @type.symbol symbol=Parser source="type Parser = typeof parse" type=(int32) => int32 & (string) => string
/// @definition.type symbol=Parser source="type Parser = typeof parse" value=(int32) => int32 & (string) => string
/// @resolution.name source=parse target=[parse#1, parse#2]

declare const parser: Parser;
/// @type.symbol symbol=parser source=parser type=Parser
/// @resolution.pattern source=parser kind=binding target=parser
/// @resolution.name source=Parser target=Parser

parser satisfies ((value: int32) => int32) & ((value: string) => string);
/// @resolution.name source=parser target=parser
/// @resolution.place source=parser placement="local" lifetime="static" access="immutable"
/// @resolution.access source=parser root=parser
/// @type.symbol symbol=value#1 source="value: int32" type=int32
/// @type.symbol symbol=value#2 source="value: string" type=string
"#,
        r#"
"#,
    );
}
