use crate::tests::{DirRows, TestSession};

#[test]
fn test_explicit_cast_coerces_literal_to_target_type() {
    let session = TestSession::single(
        r#"
const value = 1 as int32;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types().with_coercion(),
        r#"
=== annotated ===
const value: int32 = 1 as int32;

=== dir ===
const value = 1 as int32;
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="1 as int32" type=int32
/// @type.node source=1 type=1
/// @coercion.node source=1 from=1 adjustments=[{ kind: materialize, target: int32 }] origin=explicit
"#,
    );
}

/// A cast widens scalars losslessly across domains and pointer widths.
#[test]
fn test_cast_widens_scalars_losslessly() {
    let session = TestSession::single(
        r#"
declare const small: int8;
declare const byte: uint8;
declare const count: uint32;
declare const offset: isize;
declare const letter: char;

const wide = small as int32;
const signed = byte as int16;
const real = count as float64;
const index = count as usize;
const large = offset as int64;
const code = letter as uint32;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
declare const small: int8;
declare const byte: uint8;
declare const count: uint32;
declare const offset: isize;
declare const letter: char;

const wide: int32 = small as int32;
const signed: int16 = byte as int16;
const real: float64 = count as float64;
const index: usize = count as usize;
const large: int64 = offset as int64;
const code: uint32 = letter as uint32;

=== dir ===
declare const small: int8;
/// @type.symbol symbol=small source=small type=int8
/// @resolution.pattern source=small kind=binding target=small

declare const byte: uint8;
/// @type.symbol symbol=byte source=byte type=uint8
/// @resolution.pattern source=byte kind=binding target=byte

declare const count: uint32;
/// @type.symbol symbol=count source=count type=uint32
/// @resolution.pattern source=count kind=binding target=count

declare const offset: isize;
/// @type.symbol symbol=offset source=offset type=isize
/// @resolution.pattern source=offset kind=binding target=offset

declare const letter: char;
/// @type.symbol symbol=letter source=letter type=char
/// @resolution.pattern source=letter kind=binding target=letter

const wide = small as int32;
/// @type.symbol symbol=wide source=wide type=int32
/// @resolution.pattern source=wide kind=binding target=wide
/// @resolution.name source=small target=small
/// @resolution.place source=small placement="local" lifetime="static" access="immutable"
/// @resolution.access source=small root=small

const signed = byte as int16;
/// @type.symbol symbol=signed source=signed type=int16
/// @resolution.pattern source=signed kind=binding target=signed
/// @resolution.name source=byte target=byte
/// @resolution.place source=byte placement="local" lifetime="static" access="immutable"
/// @resolution.access source=byte root=byte

const real = count as float64;
/// @type.symbol symbol=real source=real type=float64
/// @resolution.pattern source=real kind=binding target=real
/// @resolution.name source=count target=count
/// @resolution.place source=count placement="local" lifetime="static" access="immutable"
/// @resolution.access source=count root=count

const index = count as usize;
/// @type.symbol symbol=index source=index type=usize
/// @resolution.pattern source=index kind=binding target=index
/// @resolution.name source=count target=count
/// @resolution.place source=count placement="local" lifetime="static" access="immutable"
/// @resolution.access source=count root=count

const large = offset as int64;
/// @type.symbol symbol=large source=large type=int64
/// @resolution.pattern source=large kind=binding target=large
/// @resolution.name source=offset target=offset
/// @resolution.place source=offset placement="local" lifetime="static" access="immutable"
/// @resolution.access source=offset root=offset

const code = letter as uint32;
/// @type.symbol symbol=code source=code type=uint32
/// @resolution.pattern source=code kind=binding target=code
/// @resolution.name source=letter target=letter
/// @resolution.place source=letter placement="local" lifetime="static" access="immutable"
/// @resolution.access source=letter root=letter
"#,
        r#"
"#,
    );
}

/// A cast rejects lossy scalar conversions.
#[test]
fn test_cast_rejects_lossy_scalar_conversion() {
    let session = TestSession::single(
        r#"
declare const total: int64;
declare const ratio: float64;
declare const count: uint32;
declare const index: usize;

const narrow = total as int8;
const flip = count as int32;
const inexact = total as float64;
const shrunk = ratio as float32;
const reindexed = index as isize;
const whole = ratio as int32;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
declare const total: int64;
declare const ratio: float64;
declare const count: uint32;
declare const index: usize;

const narrow: int8 = total as int8;
const flip: int32 = count as int32;
const inexact: float64 = total as float64;
const shrunk: float32 = ratio as float32;
const reindexed: isize = index as isize;
const whole: int32 = ratio as int32;

=== dir ===
declare const total: int64;
/// @type.symbol symbol=total source=total type=int64
/// @resolution.pattern source=total kind=binding target=total

declare const ratio: float64;
/// @type.symbol symbol=ratio source=ratio type=float64
/// @resolution.pattern source=ratio kind=binding target=ratio

declare const count: uint32;
/// @type.symbol symbol=count source=count type=uint32
/// @resolution.pattern source=count kind=binding target=count

declare const index: usize;
/// @type.symbol symbol=index source=index type=usize
/// @resolution.pattern source=index kind=binding target=index

const narrow = total as int8;
/// @type.symbol symbol=narrow source=narrow type=int8
/// @resolution.pattern source=narrow kind=binding target=narrow
/// @resolution.name source=total target=total
/// @resolution.place source=total placement="local" lifetime="static" access="immutable"
/// @resolution.access source=total root=total

const flip = count as int32;
/// @type.symbol symbol=flip source=flip type=int32
/// @resolution.pattern source=flip kind=binding target=flip
/// @resolution.name source=count target=count
/// @resolution.place source=count placement="local" lifetime="static" access="immutable"
/// @resolution.access source=count root=count

const inexact = total as float64;
/// @type.symbol symbol=inexact source=inexact type=float64
/// @resolution.pattern source=inexact kind=binding target=inexact
/// @resolution.name source=total target=total
/// @resolution.place source=total placement="local" lifetime="static" access="immutable"
/// @resolution.access source=total root=total

const shrunk = ratio as float32;
/// @type.symbol symbol=shrunk source=shrunk type=float32
/// @resolution.pattern source=shrunk kind=binding target=shrunk
/// @resolution.name source=ratio target=ratio
/// @resolution.place source=ratio placement="local" lifetime="static" access="immutable"
/// @resolution.access source=ratio root=ratio

const reindexed = index as isize;
/// @type.symbol symbol=reindexed source=reindexed type=isize
/// @resolution.pattern source=reindexed kind=binding target=reindexed
/// @resolution.name source=index target=index
/// @resolution.place source=index placement="local" lifetime="static" access="immutable"
/// @resolution.access source=index root=index

const whole = ratio as int32;
/// @type.symbol symbol=whole source=whole type=int32
/// @resolution.pattern source=whole kind=binding target=whole
/// @resolution.name source=ratio target=ratio
/// @resolution.place source=ratio placement="local" lifetime="static" access="immutable"
/// @resolution.access source=ratio root=ratio
"#,
        r#"
/// @diagnostic.error id=invalid-cast message="type 'int64' cannot be cast to 'int8'"
/// @diagnostic.label line=7 column=16 span="total" line_source="const narrow = total as int8;"
/// @diagnostic.error id=invalid-cast message="type 'uint32' cannot be cast to 'int32'"
/// @diagnostic.label line=8 column=14 span="count" line_source="const flip = count as int32;"
/// @diagnostic.error id=invalid-cast message="type 'int64' cannot be cast to 'float64'"
/// @diagnostic.label line=9 column=17 span="total" line_source="const inexact = total as float64;"
/// @diagnostic.error id=invalid-cast message="type 'float64' cannot be cast to 'float32'"
/// @diagnostic.label line=10 column=16 span="ratio" line_source="const shrunk = ratio as float32;"
/// @diagnostic.error id=invalid-cast message="type 'usize' cannot be cast to 'isize'"
/// @diagnostic.label line=11 column=19 span="index" line_source="const reindexed = index as isize;"
/// @diagnostic.error id=invalid-cast message="type 'float64' cannot be cast to 'int32'"
/// @diagnostic.label line=12 column=15 span="ratio" line_source="const whole = ratio as int32;"
"#,
    );
}

/// A cast rejects partial conversions out of a union or across domains.
#[test]
fn test_cast_rejects_a_partial_conversion() {
    let session = TestSession::single(
        r#"
declare const value: int32 | string;
declare const flag: boolean;

const exit = value as int32;
const bit = flag as int32;
const parsed = "1" as int32;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
declare const value: int32 | string;
declare const flag: boolean;

const exit: int32 = value as int32;
const bit: int32 = flag as int32;
const parsed: int32 = "1" as int32;

=== dir ===
declare const value: int32 | string;
/// @type.symbol symbol=value source=value type=int32 | string
/// @resolution.pattern source=value kind=binding target=value

declare const flag: boolean;
/// @type.symbol symbol=flag source=flag type=boolean
/// @resolution.pattern source=flag kind=binding target=flag

const exit = value as int32;
/// @type.symbol symbol=exit source=exit type=int32
/// @resolution.pattern source=exit kind=binding target=exit
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value

const bit = flag as int32;
/// @type.symbol symbol=bit source=bit type=int32
/// @resolution.pattern source=bit kind=binding target=bit
/// @resolution.name source=flag target=flag
/// @resolution.place source=flag placement="local" lifetime="static" access="immutable"
/// @resolution.access source=flag root=flag

const parsed = "1" as int32;
/// @type.symbol symbol=parsed source=parsed type=int32
/// @resolution.pattern source=parsed kind=binding target=parsed
"#,
        r#"
/// @diagnostic.error id=invalid-cast message="type 'int32 | string' cannot be cast to 'int32'"
/// @diagnostic.label line=5 column=14 span="value" line_source="const exit = value as int32;"
/// @diagnostic.note message="expected 'int32', found 'string'"
/// @diagnostic.error id=invalid-cast message="type 'boolean' cannot be cast to 'int32'"
/// @diagnostic.label line=6 column=13 span="flag" line_source="const bit = flag as int32;"
/// @diagnostic.error id=invalid-cast message="type '\"1\"' cannot be cast to 'int32'"
/// @diagnostic.label line=7 column=16 span="\"1\"" line_source="const parsed = \"1\" as int32;"
"#,
    );
}
