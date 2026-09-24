use crate::tests::{DirRows, TestSession};

/// Stage intrinsic equality for builtin scalars and declared equality for library scalars.
#[test]
fn test_resolve_scalar_equality_protocols() {
    let session = TestSession::single(
        r#"
import { Equal, PartialEqual } from "destack:ops";

declare function requireEqual<T: Equal<T>>(value: T): void;
declare function requirePartialEqual<T: PartialEqual<T>>(value: T): void;

declare const booleanValue: boolean;
declare const characterValue: char;
declare const integerValue: int32;
declare const floatValue: float64;
declare const stringValue: string;
declare const bigintValue: bigint;

requireEqual(booleanValue);
requireEqual(characterValue);
requireEqual(integerValue);
requirePartialEqual(floatValue);
requireEqual(stringValue);
requireEqual(bigintValue);
requireEqual(true);
requireEqual('x');
requireEqual(1 as int32);
requirePartialEqual(1.0);
requireEqual(null);
requireEqual(undefined);
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
import { Equal, PartialEqual } from "destack:ops";

declare function requireEqual<T: Equal<T>>(value: T): void;
declare function requirePartialEqual<T: PartialEqual<T>>(value: T): void;

declare const booleanValue: boolean;
declare const characterValue: char;
declare const integerValue: int32;
declare const floatValue: float64;
declare const stringValue: string;
declare const bigintValue: bigint;

requireEqual<boolean>(booleanValue);
requireEqual<char>(characterValue);
requireEqual<int32>(integerValue);
requirePartialEqual<float64>(floatValue);
requireEqual<string>(stringValue);
requireEqual<bigint>(bigintValue);
requireEqual<boolean>(true);
requireEqual<char>('x');
requireEqual<int32>(1 as int32);
requirePartialEqual<float64>(1.0);
requireEqual<null>(null);
requireEqual<undefined>(undefined);

=== dir ===
import { Equal, PartialEqual } from "destack:ops";

declare function requireEqual<T: Equal<T>>(value: T): void;
/// @generic.template symbol=requireEqual parameters=(T#1: Equal<T#1>)
/// @type.symbol symbol=requireEqual source="declare function requireEqual<T: Equal<T>>(value: T): void" type=<T#1: Equal<T#1>>(T#1) => void
/// @type.symbol symbol=requireEqual.T source="T: Equal<T>" type=T#1
/// @resolution.name source=Equal target=Equal
/// @resolution.name source=T target=requireEqual.T
/// @resolution.name source=T target=requireEqual.T

declare function requirePartialEqual<T: PartialEqual<T>>(value: T): void;
/// @generic.template symbol=requirePartialEqual parameters=(T#2: PartialEqual<T#2>)
/// @type.symbol symbol=requirePartialEqual source="declare function requirePartialEqual<T: PartialEqual<T>>(value: T): void" type=<T#2: PartialEqual<T#2>>(T#2) => void
/// @type.symbol symbol=requirePartialEqual.T source="T: PartialEqual<T>" type=T#2
/// @resolution.name source=PartialEqual target=PartialEqual
/// @resolution.name source=T target=requirePartialEqual.T
/// @resolution.name source=T target=requirePartialEqual.T

declare const booleanValue: boolean;
/// @type.symbol symbol=booleanValue source=booleanValue type=boolean
/// @resolution.pattern source=booleanValue kind=binding target=booleanValue

declare const characterValue: char;
/// @type.symbol symbol=characterValue source=characterValue type=char
/// @resolution.pattern source=characterValue kind=binding target=characterValue

declare const integerValue: int32;
/// @type.symbol symbol=integerValue source=integerValue type=int32
/// @resolution.pattern source=integerValue kind=binding target=integerValue

declare const floatValue: float64;
/// @type.symbol symbol=floatValue source=floatValue type=float64
/// @resolution.pattern source=floatValue kind=binding target=floatValue

declare const stringValue: string;
/// @type.symbol symbol=stringValue source=stringValue type=string
/// @resolution.pattern source=stringValue kind=binding target=stringValue

declare const bigintValue: bigint;
/// @type.symbol symbol=bigintValue source=bigintValue type=bigint
/// @resolution.pattern source=bigintValue kind=binding target=bigintValue

requireEqual(booleanValue);
/// @resolution.name source=requireEqual target=requireEqual
/// @resolution.call source=requireEqual(booleanValue) parameters=(boolean) arguments=(provided(booleanValue) as boolean) return=void kind=symbol target=requireEqual instance=requireEqual<boolean>
/// @generic.instantiation id=requireEqual<boolean> template=requireEqual arguments=(boolean)
/// @resolution.name source=booleanValue target=booleanValue
/// @resolution.place source=booleanValue placement="local" lifetime="static" access="immutable"
/// @resolution.access source=booleanValue root=booleanValue

requireEqual(characterValue);
/// @resolution.name source=requireEqual target=requireEqual
/// @resolution.call source=requireEqual(characterValue) parameters=(char) arguments=(provided(characterValue) as char) return=void kind=symbol target=requireEqual instance=requireEqual<char>
/// @generic.instantiation id=requireEqual<char> template=requireEqual arguments=(char)
/// @resolution.name source=characterValue target=characterValue
/// @resolution.place source=characterValue placement="local" lifetime="static" access="immutable"
/// @resolution.access source=characterValue root=characterValue

requireEqual(integerValue);
/// @resolution.name source=requireEqual target=requireEqual
/// @resolution.call source=requireEqual(integerValue) parameters=(int32) arguments=(provided(integerValue) as int32) return=void kind=symbol target=requireEqual instance=requireEqual<int32>
/// @generic.instantiation id=requireEqual<int32> template=requireEqual arguments=(int32)
/// @resolution.name source=integerValue target=integerValue
/// @resolution.place source=integerValue placement="local" lifetime="static" access="immutable"
/// @resolution.access source=integerValue root=integerValue

requirePartialEqual(floatValue);
/// @resolution.name source=requirePartialEqual target=requirePartialEqual
/// @resolution.call source=requirePartialEqual(floatValue) parameters=(float64) arguments=(provided(floatValue) as float64) return=void kind=symbol target=requirePartialEqual instance=requirePartialEqual<float64>
/// @generic.instantiation id=requirePartialEqual<float64> template=requirePartialEqual arguments=(float64)
/// @resolution.name source=floatValue target=floatValue
/// @resolution.place source=floatValue placement="local" lifetime="static" access="immutable"
/// @resolution.access source=floatValue root=floatValue

requireEqual(stringValue);
/// @resolution.name source=requireEqual target=requireEqual
/// @resolution.call source=requireEqual(stringValue) parameters=(string) arguments=(provided(stringValue) as string) return=void kind=symbol target=requireEqual instance=requireEqual<string>
/// @generic.instantiation id=requireEqual<string> template=requireEqual arguments=(string)
/// @resolution.name source=stringValue target=stringValue
/// @resolution.place source=stringValue placement="local" lifetime="static" access="immutable"
/// @resolution.access source=stringValue root=stringValue

requireEqual(bigintValue);
/// @resolution.name source=requireEqual target=requireEqual
/// @resolution.call source=requireEqual(bigintValue) parameters=(bigint) arguments=(provided(bigintValue) as bigint) return=void kind=symbol target=requireEqual instance=requireEqual<bigint>
/// @generic.instantiation id=requireEqual<bigint> template=requireEqual arguments=(bigint)
/// @resolution.name source=bigintValue target=bigintValue
/// @resolution.place source=bigintValue placement="local" lifetime="static" access="immutable"
/// @resolution.access source=bigintValue root=bigintValue

requireEqual(true);
/// @resolution.name source=requireEqual target=requireEqual
/// @resolution.call source=requireEqual(true) parameters=(boolean) arguments=(provided(true) as boolean) return=void kind=symbol target=requireEqual instance=requireEqual<boolean>

requireEqual('x');
/// @resolution.name source=requireEqual target=requireEqual
/// @resolution.call source=requireEqual('x') parameters=(char) arguments=(provided('x') as char) return=void kind=symbol target=requireEqual instance=requireEqual<char>

requireEqual(1 as int32);
/// @resolution.name source=requireEqual target=requireEqual
/// @resolution.call source="requireEqual(1 as int32)" parameters=(int32) arguments=(provided(1 as int32) as int32) return=void kind=symbol target=requireEqual instance=requireEqual<int32>

requirePartialEqual(1.0);
/// @resolution.name source=requirePartialEqual target=requirePartialEqual
/// @resolution.call source=requirePartialEqual(1.0) parameters=(float64) arguments=(provided(1.0) as float64) return=void kind=symbol target=requirePartialEqual instance=requirePartialEqual<float64>

requireEqual(null);
/// @resolution.name source=requireEqual target=requireEqual
/// @resolution.call source=requireEqual(null) parameters=(null) arguments=(provided(null) as null) return=void kind=symbol target=requireEqual instance=requireEqual<null>
/// @generic.instantiation id=requireEqual<null> template=requireEqual arguments=(null)

requireEqual(undefined);
/// @resolution.name source=requireEqual target=requireEqual
/// @resolution.call source=requireEqual(undefined) parameters=(undefined) arguments=(provided(undefined) as undefined) return=void kind=symbol target=requireEqual instance=requireEqual<undefined>
/// @generic.instantiation id=requireEqual<undefined> template=requireEqual arguments=(undefined)
"#, r#"
"#);
}

/// Keep floating point equality partial because NaN is not equal to itself.
#[test]
fn test_reject_total_float_equality() {
    let session = TestSession::single(
        r#"
import { Equal } from "destack:ops";

declare function requireEqual<T: Equal<T>>(value: T): void;
declare const value: float64;

requireEqual(value);
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
import { Equal } from "destack:ops";

declare function requireEqual<T: Equal<T>>(value: T): void;
declare const value: float64;

requireEqual<float64>(value);

=== dir ===
import { Equal } from "destack:ops";

declare function requireEqual<T: Equal<T>>(value: T): void;
/// @generic.template symbol=requireEqual parameters=(T: Equal<T>)
/// @type.symbol symbol=requireEqual source="declare function requireEqual<T: Equal<T>>(value: T): void" type=<T: Equal<T>>(T) => void
/// @type.symbol symbol=requireEqual.T source="T: Equal<T>" type=T
/// @resolution.name source=Equal target=Equal
/// @resolution.name source=T target=requireEqual.T
/// @resolution.name source=T target=requireEqual.T

declare const value: float64;
/// @type.symbol symbol=value source=value type=float64
/// @resolution.pattern source=value kind=binding target=value

requireEqual(value);
/// @resolution.name source=requireEqual target=requireEqual
/// @resolution.call source=requireEqual(value) parameters=(float64) arguments=(provided(value) as float64) return=void kind=symbol target=requireEqual instance=requireEqual<float64>
/// @generic.instantiation id=requireEqual<float64> template=requireEqual arguments=(float64)
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value
"#,
        r#"
/// @diagnostic.error id=constraint-not-satisfied message="type 'float64' does not satisfy 'Equal<float64>'"
/// @diagnostic.label line=7 column=1 span="requireEqual(value)" line_source="requireEqual(value);"
/// @diagnostic.related line=4 column=31 span="T" line_source="declare function requireEqual<T: Equal<T>>(value: T): void;" message="required by this bound on 'T'"
"#,
    );
}

/// Reject cross type equality that has no declared implementation.
#[test]
fn test_reject_intrinsic_cross_type_equality() {
    let session = TestSession::single(
        r#"
import { PartialEqual } from "destack:ops";

declare function requireStringEqual<T: PartialEqual<string>>(value: T): void;
declare const value: int32;

requireStringEqual(value);
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
import { PartialEqual } from "destack:ops";

declare function requireStringEqual<T: PartialEqual<string>>(value: T): void;
declare const value: int32;

requireStringEqual<int32>(value);

=== dir ===
import { PartialEqual } from "destack:ops";

declare function requireStringEqual<T: PartialEqual<string>>(value: T): void;
/// @generic.template symbol=requireStringEqual parameters=(T: PartialEqual<string>)
/// @type.symbol symbol=requireStringEqual source="declare function requireStringEqual<T: PartialEqual<string>>(value: T): void" type=<T: PartialEqual<string>>(T) => void
/// @type.symbol symbol=requireStringEqual.T source="T: PartialEqual<string>" type=T
/// @resolution.name source=PartialEqual target=PartialEqual
/// @resolution.name source=T target=requireStringEqual.T

declare const value: int32;
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value

requireStringEqual(value);
/// @resolution.name source=requireStringEqual target=requireStringEqual
/// @resolution.call source=requireStringEqual(value) parameters=(int32) arguments=(provided(value) as int32) return=void kind=symbol target=requireStringEqual instance=requireStringEqual<int32>
/// @generic.instantiation id=requireStringEqual<int32> template=requireStringEqual arguments=(int32)
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value
"#,
        r#"
/// @diagnostic.error id=constraint-not-satisfied message="type 'int32' does not satisfy 'PartialEqual<string>'"
/// @diagnostic.label line=7 column=1 span="requireStringEqual(value)" line_source="requireStringEqual(value);"
/// @diagnostic.related line=4 column=37 span="T" line_source="declare function requireStringEqual<T: PartialEqual<string>>(value: T): void;" message="required by this bound on 'T'"
"#,
    );
}
