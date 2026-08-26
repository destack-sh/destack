use crate::tests::{DirRows, TestSession};

/// Resolve scalar and borrowed string membership through their matching protocols.
#[test]
fn test_resolve_collection_membership() {
    let session = TestSession::single(
        r#"
import { Array, Map, Set } from "destack:collections";
import { StringSlice } from "destack:string";

declare const integers: Array<int32>;
declare const integer: int32;
declare const floats: Array<float64>;
declare const float: float64;
declare const strings: Array<string>;
declare const stringValue: string;
declare const stringSlice: &readonly StringSlice;
declare const stringMap: Map<string, int32>;
declare const stringSet: Set<string>;

integers.includes(integer);
floats.includes(float);
strings.includes(stringValue);
strings.includes(stringSlice);
stringMap.has(stringSlice);
stringSet.has(stringSlice);
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
import { Array, Map, Set } from "destack:collections";
import { StringSlice } from "destack:string";

declare const integers: int32[];
declare const integer: int32;
declare const floats: float64[];
declare const float: float64;
declare const strings: string[];
declare const stringValue: string;
declare const stringSlice: &'static readonly StringSlice;
declare const stringMap: Map<string, int32>;
declare const stringSet: Set<string>;

integers.includes<int32, int32, "local", "constant">(integer as &'static readonly int32);
floats.includes<float64, float64, "local", "constant">(float as &'static readonly float64);
strings.includes<string, string, "local", "local">(stringValue as &'static readonly string);
strings.includes<string, StringSlice, "local", "constant">(stringSlice);
stringMap.has<string, int32, StringSlice, "local", "constant">(stringSlice);
stringSet.has<string, StringSlice, "local", "constant">(stringSlice);

=== dir ===
import { Array, Map, Set } from "destack:collections";
import { StringSlice } from "destack:string";

declare const integers: Array<int32>;
/// @type.symbol symbol=integers source=integers type=int32[]
/// @resolution.pattern source=integers kind=binding target=integers
/// @resolution.name source=Array target=Array

declare const integer: int32;
/// @type.symbol symbol=integer source=integer type=int32
/// @resolution.pattern source=integer kind=binding target=integer

declare const floats: Array<float64>;
/// @type.symbol symbol=floats source=floats type=float64[]
/// @resolution.pattern source=floats kind=binding target=floats
/// @resolution.name source=Array target=Array

declare const float: float64;
/// @type.symbol symbol=float source=float type=float64
/// @resolution.pattern source=float kind=binding target=float

declare const strings: Array<string>;
/// @type.symbol symbol=strings source=strings type=string[]
/// @resolution.pattern source=strings kind=binding target=strings
/// @resolution.name source=Array target=Array

declare const stringValue: string;
/// @type.symbol symbol=stringValue source=stringValue type=string
/// @resolution.pattern source=stringValue kind=binding target=stringValue

declare const stringSlice: &readonly StringSlice;
/// @type.symbol symbol=stringSlice source=stringSlice type=&'static readonly constant StringSlice
/// @resolution.pattern source=stringSlice kind=binding target=stringSlice
/// @resolution.name source=StringSlice target=StringSlice

declare const stringMap: Map<string, int32>;
/// @type.symbol symbol=stringMap source=stringMap type=Map<string, int32>
/// @resolution.pattern source=stringMap kind=binding target=stringMap
/// @resolution.name source=Map target=Map

declare const stringSet: Set<string>;
/// @type.symbol symbol=stringSet source=stringSet type=Set<string>
/// @resolution.pattern source=stringSet kind=binding target=stringSet
/// @resolution.name source=Set target=Set

integers.includes(integer);
/// @resolution.name source=integers target=integers
/// @resolution.member source=integers.includes receiver=int32[] type=<includes.Q, includes.'a, includes.P2: Place, includes.'b, includes.P4: Place>(this: Borrowed<int32[], includes.'a & includes.P2, "readonly">, &includes.'b readonly includes.Q, isize | undefined?) => boolean kind=symbol target_receiver=int32[] target=includes
/// @resolution.call source=integers.includes(integer) parameters=(&'static readonly constant int32, isize | undefined) arguments=(provided(integer) as &'static readonly constant int32, omitted as isize | undefined) return=boolean kind=symbol target=includes receiver=int32[] adjustments=(borrow(&'static readonly int32[])) instance="Array<int32>.<extension#5>.includes<int32, \"local\", \"constant\">"
/// @resolution.place source=integers placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=integers root=integers
/// @generic.instantiation id="includes<int32, int32, \"local\", \"constant\">" template=includes arguments=(int32, int32, "local", "constant")
/// @generic.instantiation id=includes<int32> template=includes arguments=(int32)
/// @resolution.name source=integer target=integer
/// @resolution.place source=integer placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=integer root=integer

floats.includes(float);
/// @resolution.name source=floats target=floats
/// @resolution.member source=floats.includes receiver=float64[] type=<includes.Q, includes.'a, includes.P2: Place, includes.'b, includes.P4: Place>(this: Borrowed<float64[], includes.'a & includes.P2, "readonly">, &includes.'b readonly includes.Q, isize | undefined?) => boolean kind=symbol target_receiver=float64[] target=includes
/// @resolution.call source=floats.includes(float) parameters=(&'static readonly constant float64, isize | undefined) arguments=(provided(float) as &'static readonly constant float64, omitted as isize | undefined) return=boolean kind=symbol target=includes receiver=float64[] adjustments=(borrow(&'static readonly float64[])) instance="Array<float64>.<extension#5>.includes<float64, \"local\", \"constant\">"
/// @resolution.place source=floats placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=floats root=floats
/// @generic.instantiation id="includes<float64, float64, \"local\", \"constant\">" template=includes arguments=(float64, float64, "local", "constant")
/// @generic.instantiation id=includes<float64> template=includes arguments=(float64)
/// @resolution.name source=float target=float
/// @resolution.place source=float placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=float root=float

strings.includes(stringValue);
/// @resolution.name source=strings target=strings
/// @resolution.member source=strings.includes receiver=string[] type=<includes.Q, includes.'a, includes.P2: Place, includes.'b, includes.P4: Place>(this: Borrowed<string[], includes.'a & includes.P2, "readonly">, &includes.'b readonly includes.Q, isize | undefined?) => boolean kind=symbol target_receiver=string[] target=includes
/// @resolution.call source=strings.includes(stringValue) parameters=(&'static readonly string, isize | undefined) arguments=(provided(stringValue) as &'static readonly string, omitted as isize | undefined) return=boolean kind=symbol target=includes receiver=string[] adjustments=(borrow(&'static readonly string[])) instance="Array<string>.<extension#5>.includes<string, \"local\", \"local\">"
/// @resolution.place source=strings placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=strings root=strings
/// @generic.instantiation id="includes<string, string, \"local\", \"local\">" template=includes arguments=(string, string, "local", "local")
/// @generic.instantiation id=includes<string> template=includes arguments=(string)
/// @resolution.name source=stringValue target=stringValue
/// @resolution.place source=stringValue placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=stringValue root=stringValue

strings.includes(stringSlice);
/// @resolution.name source=strings target=strings
/// @resolution.member source=strings.includes receiver=string[] type=<includes.Q, includes.'a, includes.P2: Place, includes.'b, includes.P4: Place>(this: Borrowed<string[], includes.'a & includes.P2, "readonly">, &includes.'b readonly includes.Q, isize | undefined?) => boolean kind=symbol target_receiver=string[] target=includes
/// @resolution.call source=strings.includes(stringSlice) parameters=(&'static readonly constant StringSlice, isize | undefined) arguments=(provided(stringSlice) as &'static readonly constant StringSlice, omitted as isize | undefined) return=boolean kind=symbol target=includes receiver=string[] adjustments=(borrow(&'static readonly string[])) instance="Array<string>.<extension#5>.includes<StringSlice, \"local\", \"constant\">"
/// @resolution.place source=strings placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=strings root=strings
/// @generic.instantiation id="includes<string, StringSlice, \"local\", \"constant\">" template=includes arguments=(string, StringSlice, "local", "constant")
/// @resolution.name source=stringSlice target=stringSlice
/// @resolution.place source=stringSlice placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=stringSlice root=stringSlice

stringMap.has(stringSlice);
/// @resolution.name source=stringMap target=stringMap
/// @resolution.member source=stringMap.has receiver=Map<string, int32> type=<has.Q: Hash, has.'a, has.P2: Place, has.'b, has.P4: Place>(this: Borrowed<Map<string, int32>, has.'a & has.P2, "readonly">, &has.'b readonly has.Q) => boolean kind=symbol target_receiver=Map<string, int32> target=has
/// @resolution.call source=stringMap.has(stringSlice) parameters=(&'static readonly constant StringSlice) arguments=(provided(stringSlice) as &'static readonly constant StringSlice) return=boolean kind=symbol target=has receiver=Map<string, int32> adjustments=(borrow(&'static readonly Map<string, int32>)) instance="Map<string, int32>.<extension#8>.has<StringSlice, \"local\", \"constant\">"
/// @resolution.place source=stringMap placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=stringMap root=stringMap
/// @generic.instantiation id="has<string, int32, StringSlice, \"local\", \"constant\">" template=has arguments=(string, int32, StringSlice, "local", "constant")
/// @generic.instantiation id="has<string, int32>" template=has arguments=(string, int32)
/// @resolution.name source=stringSlice target=stringSlice
/// @resolution.place source=stringSlice placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=stringSlice root=stringSlice

stringSet.has(stringSlice);
/// @resolution.name source=stringSet target=stringSet
/// @resolution.member source=stringSet.has receiver=Set<string> type=<has.Q: Hash, has.'a, has.P2: Place, has.'b, has.P4: Place>(this: Borrowed<Set<string>, has.'a & has.P2, "readonly">, &has.'b readonly has.Q) => boolean kind=symbol target_receiver=Set<string> target=has
/// @resolution.call source=stringSet.has(stringSlice) parameters=(&'static readonly constant StringSlice) arguments=(provided(stringSlice) as &'static readonly constant StringSlice) return=boolean kind=symbol target=has receiver=Set<string> adjustments=(borrow(&'static readonly Set<string>)) instance="Set<string>.<extension#5>.has<StringSlice, \"local\", \"constant\">"
/// @resolution.place source=stringSet placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=stringSet root=stringSet
/// @generic.instantiation id="has<string, StringSlice, \"local\", \"constant\">" template=has arguments=(string, StringSlice, "local", "constant")
/// @generic.instantiation id=has<string> template=has arguments=(string)
/// @resolution.name source=stringSlice target=stringSlice
/// @resolution.place source=stringSlice placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=stringSlice root=stringSlice
"#, "");
}
