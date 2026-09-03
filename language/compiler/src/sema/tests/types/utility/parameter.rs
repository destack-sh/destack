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
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Args = Parameters<(name: string, count: float64) => boolean>;

const ok: (string, float64) = ("Ada", 1);
ok satisfies (string, number);

=== dir ===
type Args = Parameters<(name: string, count: number) => boolean>;
/// @type.symbol symbol=Args source="type Args = Parameters<(name: string, count: number) => boolean>" type=(string, float64)
/// @definition.type symbol=Args source="type Args = Parameters<(name: string, count: number) => boolean>" value=(string, float64)
/// @resolution.name source=Parameters target=Parameters
/// @type.symbol symbol=Args.name source="name: string" type=string
/// @type.symbol symbol=Args.count source="count: number" type=float64

const ok: Args = ("Ada", 1);
/// @type.symbol symbol=ok source=ok type=(string, float64)
/// @resolution.pattern source=ok kind=binding target=ok
/// @resolution.name source=Args target=Args

ok satisfies (string, number);
/// @resolution.name source=ok target=ok
/// @resolution.place source=ok placement="constant" lifetime="static" access="readonly"
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
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Args = Parameters<(name: string, count?: float64) => boolean>;

const short: (string, float64 | undefined?) = ("Ada",) as (string, float64 | undefined?);
const full: (string, float64 | undefined?) = ("Ada", 1 as float64 | undefined) as (
    string,
    float64 | undefined?,
);

=== dir ===
type Args = Parameters<(name: string, count?: number) => boolean>;
/// @type.symbol symbol=Args source="type Args = Parameters<(name: string, count?: number) => boolean>" type=(string, float64 | undefined?)
/// @definition.type symbol=Args source="type Args = Parameters<(name: string, count?: number) => boolean>" value=(string, float64 | undefined?)
/// @resolution.name source=Parameters target=Parameters
/// @type.symbol symbol=Args.name source="name: string" type=string
/// @type.symbol symbol=Args.count source="count?: number" type=float64 | undefined

const short: Args = ("Ada",);
/// @type.symbol symbol=short source=short type=(string, float64 | undefined?)
/// @resolution.pattern source=short kind=binding target=short
/// @resolution.name source=Args target=Args

const full: Args = ("Ada", 1);
/// @type.symbol symbol=full source=full type=(string, float64 | undefined?)
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
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Args = Parameters<(name: string, ...flags: boolean[]) => void>;

const ok: (string, ...boolean[]) = ("Ada", true, false) as (string, ...boolean[]);

=== dir ===
type Args = Parameters<(name: string, ...flags: boolean[]) => void>;
/// @type.symbol symbol=Args source="type Args = Parameters<(name: string, ...flags: boolean[]) => void>" type=(string, ...boolean[])
/// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
/// @generic.instance id="elementSlot<boolean, \"exclusive\">" template=elementSlot arguments=(boolean, "exclusive") evaluated=(<elementSlot.T, const elementSlot.A: Access = "readonly", elementSlot.'a>(WithAccess<&elementSlot.'a elementSlot.T[], elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => <elementSlot.T, const elementSlot.A: Access = "readonly", elementSlot.'a>(&elementSlot.'a exclusive boolean[], usize) => &elementSlot.'a exclusive MaybeUninit<boolean>, WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => &elementSlot.'a exclusive MaybeUninit<boolean>, (WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => (&elementSlot.'a exclusive Slice<MaybeUninit<boolean>>, usize) => &elementSlot.'a exclusive MaybeUninit<boolean>, WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A> => &elementSlot.'a exclusive Slice<MaybeUninit<boolean>>, WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => &elementSlot.'a exclusive MaybeUninit<boolean>, (WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => (&elementSlot.'a exclusive Slice<MaybeUninit<boolean>>, usize) => &elementSlot.'a exclusive MaybeUninit<boolean>, WithAccess<&elementSlot.'a elementSlot.T[], elementSlot.A> => &elementSlot.'a exclusive boolean[], WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A> => &elementSlot.'a exclusive Slice<MaybeUninit<boolean>>)
/// @generic.instance id="initAsPointer<boolean, \"exclusive\">" template=initAsPointer arguments=(boolean, "exclusive") evaluated=(<initAsPointer.T, const initAsPointer.A: Access = "mutable", initAsPointer.'a>(WithAccess<&initAsPointer.'a MaybeUninit<initAsPointer.T>, initAsPointer.A>) => Raw<initAsPointer.T> => <initAsPointer.T, const initAsPointer.A: Access = "mutable", initAsPointer.'a>(&initAsPointer.'a exclusive MaybeUninit<boolean>) => Raw<boolean>)
/// @generic.instance id="sliceIndex<MaybeUninit<boolean>, \"exclusive\">" template=sliceIndex arguments=(MaybeUninit<boolean>, "exclusive") evaluated=(<sliceIndex.T, const sliceIndex.A: Access = "readonly", sliceIndex.'a>(WithAccess<&sliceIndex.'a Slice<sliceIndex.T>, sliceIndex.A>, usize) => WithAccess<&sliceIndex.'a sliceIndex.T, sliceIndex.A> => <sliceIndex.T, const sliceIndex.A: Access = "readonly", sliceIndex.'a>(&sliceIndex.'a exclusive Slice<MaybeUninit<boolean>>, usize) => &sliceIndex.'a exclusive MaybeUninit<boolean>)
/// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
/// @generic.instance id=Array<boolean> template=Array arguments=(boolean)
/// @generic.instance id=MaybeUninit<MaybeUninit<boolean>> template=MaybeUninit arguments=(MaybeUninit<boolean>)
/// @generic.instance id=MaybeUninit<boolean> template=MaybeUninit arguments=(boolean)
/// @generic.instance id=assumeInitDrop#1<boolean> template=assumeInitDrop#1 arguments=(boolean)
/// @generic.instance id=assumeInitDrop<boolean> template=assumeInitDrop arguments=(boolean) evaluated=((WithAccess<&assumeInitDrop.'a MaybeUninit<assumeInitDrop.T>, "exclusive">) => Raw<assumeInitDrop.T> => (&assumeInitDrop.'a exclusive MaybeUninit<boolean>) => Raw<boolean>, WithAccess<&assumeInitDrop.'a MaybeUninit<assumeInitDrop.T>, "exclusive"> => &assumeInitDrop.'a exclusive MaybeUninit<boolean>)
/// @generic.instance id=clear<boolean> template=clear arguments=(boolean)
/// @generic.instance id=drop<boolean> template=drop arguments=(boolean)
/// @generic.instance id=dropInPlace<boolean> template=dropInPlace arguments=(boolean)
/// @generic.instance id=new<MaybeUninit<boolean>> template=new arguments=(MaybeUninit<boolean>)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<boolean>> template=sliceAssumeInit arguments=(MaybeUninit<boolean>)
/// @generic.instance id=sliceUninit<MaybeUninit<boolean>> template=sliceUninit arguments=(MaybeUninit<boolean>)
/// @generic.instance id=truncate<boolean> template=truncate arguments=(boolean) evaluated=(WithAccess<&truncate.'a MaybeUninit<T#6>, "exclusive"> => &truncate.'a exclusive MaybeUninit<boolean>, (WithAccess<&truncate.'a T#6[], "exclusive">, usize) => WithAccess<&truncate.'a MaybeUninit<T#6>, "exclusive"> => (&truncate.'a exclusive boolean[], usize) => &truncate.'a exclusive MaybeUninit<boolean>, WithAccess<&truncate.'a T#6[], "exclusive"> => &truncate.'a exclusive boolean[])
/// @definition.type symbol=Args source="type Args = Parameters<(name: string, ...flags: boolean[]) => void>" value=(string, ...boolean[])
/// @resolution.name source=Parameters target=Parameters
/// @type.symbol symbol=Args.name source="name: string" type=string
/// @type.symbol symbol=Args.flags source="...flags: boolean[]" type=boolean[]

const ok: Args = ("Ada", true, false);
/// @type.symbol symbol=ok source=ok type=(string, ...boolean[])
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
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Args = Parameters<(name: string, count: float64) => boolean>;

const bad: (string, float64) = ("Ada", "one");

=== dir ===
type Args = Parameters<(name: string, count: number) => boolean>;
/// @type.symbol symbol=Args source="type Args = Parameters<(name: string, count: number) => boolean>" type=(string, float64)
/// @definition.type symbol=Args source="type Args = Parameters<(name: string, count: number) => boolean>" value=(string, float64)
/// @resolution.name source=Parameters target=Parameters
/// @type.symbol symbol=Args.name source="name: string" type=string
/// @type.symbol symbol=Args.count source="count: number" type=float64

const bad: Args = ("Ada", "one");
/// @type.symbol symbol=bad source=bad type=(string, float64)
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

type Parser = typeof parse;

declare const parser: ((arg0: int32) => int32) & ((arg0: string) => string);

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
    /// @resolution.place source=value placement="local" lifetime="managed" access="exclusive"
    /// @resolution.access source=value root=parse.value#2

}

type Parser = typeof parse;
/// @type.symbol symbol=Parser source="type Parser = typeof parse" type=(int32) => int32 & (string) => string
/// @definition.type symbol=Parser source="type Parser = typeof parse" value=(int32) => int32 & (string) => string
/// @resolution.name source=parse target=[parse#1, parse#2]

declare const parser: Parser;
/// @type.symbol symbol=parser source=parser type=(int32) => int32 & (string) => string
/// @resolution.pattern source=parser kind=binding target=parser
/// @resolution.name source=Parser target=Parser

parser satisfies ((value: int32) => int32) & ((value: string) => string);
/// @resolution.name source=parser target=parser
/// @resolution.place source=parser placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=parser root=parser
/// @type.symbol symbol=value#1 source="value: int32" type=int32
/// @type.symbol symbol=value#2 source="value: string" type=string
"#,
        r#"
"#,
    );
}
