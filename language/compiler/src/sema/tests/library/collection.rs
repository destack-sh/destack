use crate::tests::{DirRows, TestSession};

/// Stage scalar and borrowed string membership through their matching protocols.
#[test]
fn test_resolve_collection_membership() {
    let session = TestSession::single(
        r#"
import { Array, Map, Set } from "tspp:collections";
import { StringSlice } from "tspp:string";

declare const integers: Array<int32>;
declare const integer: int32;
declare const floats: Array<float64>;
declare const float: float64;
declare const strings: Array<string>;
declare const stringValue: string;
declare const stringSlice: &immutable StringSlice;
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

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
import { Array, Map, Set } from "tspp:collections";
import { StringSlice } from "tspp:string";

declare const integers: int32[];
declare const integer: int32;
declare const floats: float64[];
declare const float: float64;
declare const strings: string[];
declare const stringValue: string;
declare const stringSlice: &'static immutable StringSlice;
declare const stringMap: Map<string, int32, Equality<string>>;
declare const stringSet: Set<string, Equality<string>>;

integers.includes<int32, int32, "readonly", "managed", "static">(
    integer as &'static immutable int32,
);
floats.includes<float64, float64, "readonly", "managed", "static">(
    float as &'static immutable float64,
);
strings.includes<string, string, "readonly", "managed", "managed">(
    stringValue as &'managed immutable string,
);
strings.includes<string, StringSlice, "readonly", "managed", "static">(stringSlice);
stringMap.has<string, int32, Equality<string>, StringSlice, "readonly", "managed", "static">(
    stringSlice,
);
stringSet.has<string, Equality<string>, StringSlice, "readonly", "managed", "static">(stringSlice);

=== dir ===
import { Array, Map, Set } from "tspp:collections";
import { StringSlice } from "tspp:string";

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

declare const stringSlice: &immutable StringSlice;
/// @type.symbol symbol=stringSlice source=stringSlice type=&'static immutable StringSlice
/// @resolution.pattern source=stringSlice kind=binding target=stringSlice
/// @resolution.name source=StringSlice target=StringSlice

declare const stringMap: Map<string, int32>;
/// @type.symbol symbol=stringMap source=stringMap type=Map<string, int32, Equality<string>>
/// @resolution.pattern source=stringMap kind=binding target=stringMap
/// @resolution.name source=Map target=Map

declare const stringSet: Set<string>;
/// @type.symbol symbol=stringSet source=stringSet type=Set<string, Equality<string>>
/// @resolution.pattern source=stringSet kind=binding target=stringSet
/// @resolution.name source=Set target=Set

integers.includes(integer);
/// @resolution.name source=integers target=integers
/// @resolution.member source=integers.includes receiver=int32[] type=<includes.Q, const includes.A: "readonly" | "immutable", includes.'a, includes.'b>(this: WithAccess<&includes.'a int32[], includes.A>, &includes.'b immutable includes.Q, isize | undefined?) => boolean kind=symbol target_receiver=int32[] target=includes
/// @resolution.call source=integers.includes(integer) parameters=(&'static immutable int32, isize | undefined) arguments=(provided(integer) as &'static immutable int32, omitted as isize | undefined) return=boolean regions=("managed" & "local", "static" & "local") kind=symbol target=includes receiver=int32[] adjustments=(borrow(&'managed readonly int32[])) instance="Array<int32>.<extension#6>.includes<int32, \"readonly\", \"managed\" & \"local\", \"static\" & \"local\">"
/// @resolution.place source=integers placement="local" lifetime="static" access="immutable"
/// @resolution.access source=integers root=integers
/// @generic.instantiation id="includes<int32, int32, \"readonly\", \"managed\" & \"local\", \"static\" & \"local\">" template=includes arguments=(int32, int32, "readonly", "managed" & "local", "static" & "local")
/// @generic.instantiation id=includes<int32> template=includes arguments=(int32)
/// @resolution.name source=integer target=integer
/// @resolution.place source=integer placement="local" lifetime="static" access="immutable"
/// @resolution.access source=integer root=integer

floats.includes(float);
/// @resolution.name source=floats target=floats
/// @resolution.member source=floats.includes receiver=float64[] type=<includes.Q, const includes.A: "readonly" | "immutable", includes.'a, includes.'b>(this: WithAccess<&includes.'a float64[], includes.A>, &includes.'b immutable includes.Q, isize | undefined?) => boolean kind=symbol target_receiver=float64[] target=includes
/// @resolution.call source=floats.includes(float) parameters=(&'static immutable float64, isize | undefined) arguments=(provided(float) as &'static immutable float64, omitted as isize | undefined) return=boolean regions=("managed" & "local", "static" & "local") kind=symbol target=includes receiver=float64[] adjustments=(borrow(&'managed readonly float64[])) instance="Array<float64>.<extension#6>.includes<float64, \"readonly\", \"managed\" & \"local\", \"static\" & \"local\">"
/// @resolution.place source=floats placement="local" lifetime="static" access="immutable"
/// @resolution.access source=floats root=floats
/// @generic.instantiation id="includes<float64, float64, \"readonly\", \"managed\" & \"local\", \"static\" & \"local\">" template=includes arguments=(float64, float64, "readonly", "managed" & "local", "static" & "local")
/// @generic.instantiation id=includes<float64> template=includes arguments=(float64)
/// @resolution.name source=float target=float
/// @resolution.place source=float placement="local" lifetime="static" access="immutable"
/// @resolution.access source=float root=float

strings.includes(stringValue);
/// @resolution.name source=strings target=strings
/// @resolution.member source=strings.includes receiver=string[] type=<includes.Q, const includes.A: "readonly" | "immutable", includes.'a, includes.'b>(this: WithAccess<&includes.'a string[], includes.A>, &includes.'b immutable includes.Q, isize | undefined?) => boolean kind=symbol target_receiver=string[] target=includes
/// @resolution.call source=strings.includes(stringValue) parameters=(&'managed immutable string, isize | undefined) arguments=(provided(stringValue) as &'managed immutable string, omitted as isize | undefined) return=boolean regions=("managed" & "local", "managed" & "local") kind=symbol target=includes receiver=string[] adjustments=(borrow(&'managed readonly string[])) instance="Array<string>.<extension#6>.includes<string, \"readonly\", \"managed\" & \"local\", \"managed\" & \"local\">"
/// @resolution.place source=strings placement="local" lifetime="static" access="immutable"
/// @resolution.access source=strings root=strings
/// @generic.instantiation id="includes<string, string, \"readonly\", \"managed\" & \"local\", \"managed\" & \"local\">" template=includes arguments=(string, string, "readonly", "managed" & "local", "managed" & "local")
/// @generic.instantiation id=includes<string> template=includes arguments=(string)
/// @resolution.name source=stringValue target=stringValue
/// @resolution.place source=stringValue placement="local" lifetime="static" access="immutable"
/// @resolution.access source=stringValue root=stringValue

strings.includes(stringSlice);
/// @resolution.name source=strings target=strings
/// @resolution.member source=strings.includes receiver=string[] type=<includes.Q, const includes.A: "readonly" | "immutable", includes.'a, includes.'b>(this: WithAccess<&includes.'a string[], includes.A>, &includes.'b immutable includes.Q, isize | undefined?) => boolean kind=symbol target_receiver=string[] target=includes
/// @resolution.call source=strings.includes(stringSlice) parameters=(&'static immutable StringSlice, isize | undefined) arguments=(provided(stringSlice) as &'static immutable StringSlice, omitted as isize | undefined) return=boolean regions=("managed" & "local", "static" & "local") kind=symbol target=includes receiver=string[] adjustments=(borrow(&'managed readonly string[])) instance="Array<string>.<extension#6>.includes<StringSlice, \"readonly\", \"managed\" & \"local\", \"static\" & \"local\">"
/// @resolution.place source=strings placement="local" lifetime="static" access="immutable"
/// @resolution.access source=strings root=strings
/// @generic.instantiation id="includes<string, StringSlice, \"readonly\", \"managed\" & \"local\", \"static\" & \"local\">" template=includes arguments=(string, StringSlice, "readonly", "managed" & "local", "static" & "local")
/// @resolution.name source=stringSlice target=stringSlice
/// @resolution.place source=stringSlice placement="local" lifetime="static" access="immutable"
/// @resolution.access source=stringSlice root=stringSlice

stringMap.has(stringSlice);
/// @resolution.name source=stringMap target=stringMap
/// @resolution.member source=stringMap.has receiver=Map<string, int32, Equality<string>> type=<has.Q#1, const has.A#1: "readonly" | "immutable", has#1.'a>(this: WithAccess<&has#1.'a readonly Map<string, int32, Equality<string>>, has.A#1>, has.Q#1) => boolean & <has.Q#2, const has.A#2: "readonly" | "immutable", has#2.'a, has#2.'b>(this: WithAccess<&has#2.'a readonly Map<string, int32, Equality<string>>, has.A#2>, &has#2.'b immutable has.Q#2) => boolean kind=overload-set targets=[has#1, has#2]
/// @resolution.call source=stringMap.has(stringSlice) parameters=(&'static immutable StringSlice) arguments=(provided(stringSlice) as &'static immutable StringSlice) return=boolean regions=("managed" & "local", "static" & "local") kind=symbol target=has#2 receiver=Map<string, int32, Equality<string>> adjustments=(borrow(&'managed readonly Map<string, int32, Equality<string>>)) instance="Map<string, int32, Equality<string>>.<extension#9>.has#2<StringSlice, \"readonly\", \"managed\" & \"local\", \"static\" & \"local\">"
/// @resolution.place source=stringMap placement="local" lifetime="static" access="immutable"
/// @resolution.access source=stringMap root=stringMap
/// @generic.instantiation id="has#1<string, int32, Equality<string>>" template=has#1 arguments=(string, int32, Equality<string>)
/// @generic.instantiation id="has#2<string, int32, Equality<string>, StringSlice, \"readonly\", \"managed\" & \"local\", \"static\" & \"local\">" template=has#2 arguments=(string, int32, Equality<string>, StringSlice, "readonly", "managed" & "local", "static" & "local")
/// @generic.instantiation id="has#2<string, int32, Equality<string>>" template=has#2 arguments=(string, int32, Equality<string>)
/// @resolution.name source=stringSlice target=stringSlice
/// @resolution.place source=stringSlice placement="local" lifetime="static" access="immutable"
/// @resolution.access source=stringSlice root=stringSlice

stringSet.has(stringSlice);
/// @resolution.name source=stringSet target=stringSet
/// @resolution.member source=stringSet.has receiver=Set<string, Equality<string>> type=<has.Q#1, const has.A#1: "readonly" | "immutable", has#1.'a>(this: WithAccess<&has#1.'a readonly Set<string, Equality<string>>, has.A#1>, has.Q#1) => boolean & <has.Q#2, const has.A#2: "readonly" | "immutable", has#2.'a, has#2.'b>(this: WithAccess<&has#2.'a readonly Set<string, Equality<string>>, has.A#2>, &has#2.'b immutable has.Q#2) => boolean kind=overload-set targets=[has#1, has#2]
/// @resolution.call source=stringSet.has(stringSlice) parameters=(&'static immutable StringSlice) arguments=(provided(stringSlice) as &'static immutable StringSlice) return=boolean regions=("managed" & "local", "static" & "local") kind=symbol target=has#2 receiver=Set<string, Equality<string>> adjustments=(borrow(&'managed readonly Set<string, Equality<string>>)) instance="Set<string, Equality<string>>.<extension#6>.has#2<StringSlice, \"readonly\", \"managed\" & \"local\", \"static\" & \"local\">"
/// @resolution.place source=stringSet placement="local" lifetime="static" access="immutable"
/// @resolution.access source=stringSet root=stringSet
/// @generic.instantiation id="has#1<string, Equality<string>>" template=has#1 arguments=(string, Equality<string>)
/// @generic.instantiation id="has#2<string, Equality<string>, StringSlice, \"readonly\", \"managed\" & \"local\", \"static\" & \"local\">" template=has#2 arguments=(string, Equality<string>, StringSlice, "readonly", "managed" & "local", "static" & "local")
/// @generic.instantiation id="has#2<string, Equality<string>>" template=has#2 arguments=(string, Equality<string>)
/// @resolution.name source=stringSlice target=stringSlice
/// @resolution.place source=stringSlice placement="local" lifetime="static" access="immutable"
/// @resolution.access source=stringSlice root=stringSlice
"#, r#"
"#);
}

/// Clone an array through an optional immutable borrow at the borrow's own place.
#[test]
fn test_clone_an_array_through_an_optional_immutable_borrow() {
    let session = TestSession::single(
        r#"
struct Label {
    values: ^int32[];
}

function copy(values: &immutable Label[] | undefined): ^Label[] | undefined {
    return values?.clone();
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
struct Label {
    values: ^int32[];
}

function copy<'a>(values: &'a immutable Label[] | undefined): ^Label[] | undefined {
    return values?.clone<Label, 'a>() as ^Label[] | undefined;
}

=== dir ===
struct Label {
/// @type.symbol symbol=Label type=Label
/// @definition.struct symbol=Label
/// @definition.field symbol=Label.values source="values: ^int32[]" key=values type=^int32[]

    values: ^int32[];
    /// @type.symbol symbol=Label.values source="values: ^int32[]" type=^int32[]
    /// @generic.instance id=Array<int32> template=Array arguments=(int32)
    /// @generic.instance id=sliceAssumeInit<MaybeUninit<int32>> template=sliceAssumeInit arguments=(MaybeUninit<int32>)
    /// @generic.instance id=sliceUninit<MaybeUninit<int32>> template=sliceUninit arguments=(MaybeUninit<int32>)

}

function copy(values: &immutable Label[] | undefined): ^Label[] | undefined {
/// @generic.template symbol=copy parameters=('a)
/// @type.symbol symbol=copy type=<copy.'a>(&copy.'a immutable Label[] | undefined) => ^Label[] | undefined
/// @generic.instance id=Array<Label> template=Array arguments=(Label)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<Label>> template=sliceAssumeInit arguments=(MaybeUninit<Label>)
/// @generic.instance id=sliceUninit<MaybeUninit<Label>> template=sliceUninit arguments=(MaybeUninit<Label>)
/// @type.symbol symbol=copy.values source="values: &immutable Label[] | undefined" type=&copy.'a immutable Label[] | undefined
/// @resolution.name source=Label target=Label
/// @resolution.name source=Label target=Label

    return values?.clone();
    /// @resolution.name source=values target=copy.values
    /// @resolution.member source=values?.clone receiver=&copy.'a immutable Label[] | undefined type=<clone.'a>(this: &clone.'a immutable Label[]) => ^Label[] kind=symbol target_receiver=&copy.'a immutable Label[] | undefined adjustments=(union.payload(&copy.'a immutable Label[] | undefined, &copy.'a immutable Label[], &copy.'a immutable Label[])) target=clone
    /// @resolution.call source=values?.clone() parameters=() return=^Label[] regions=(copy.'a) kind=symbol target=clone receiver=&copy.'a immutable Label[] | undefined adjustments=(union.payload(&copy.'a immutable Label[] | undefined, &copy.'a immutable Label[], &copy.'a immutable Label[])) instance=Array<Label>.<extension#1>.clone<copy.'a>
    /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values root=copy.values
    /// @generic.instantiation id="clone<Label, copy.'a>" template=clone arguments=(Label, copy.'a)
    /// @generic.instantiation id=clone<Label> template=clone arguments=(Label)
    /// @generic.instance id="clone<Label, copy.'a>" template=clone arguments=(Label, copy.'a)
    /// @generic.instance id=Clone.clone<Label> template=Clone.clone arguments=()

}

/// @generic.template symbol=Clone.clone parameters=('a)
/// @type.symbol symbol=Clone.clone type=<Clone.clone.'a>(this: &Clone.clone.'a immutable Label) => ^Label
/// @generic.instance id="clone<int32, Clone.clone.'a>" template=clone arguments=(int32, Clone.clone.'a)
"#,
    );
}

/// Clone a borrowed iterator into an array.
#[test]
fn test_clone_a_borrowed_iterator_into_an_array() {
    let session = TestSession::single(
        r#"
import { Iterator } from "tspp:iter";

declare function values(): Iterator<&immutable int32>;

const copied = values().cloned().toArray();
copied satisfies int32[];
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { Iterator } from "tspp:iter";

declare function values(): Iterator<&immutable int32>;

const copied: ^int32[] = values().cloned<&'managed immutable int32, int32, "managed", "immutable">()
    .toArray<^int32, int32, "managed", "immutable", Iterator<&'managed immutable int32>>();
copied satisfies int32[];

=== dir ===
import { Iterator } from "tspp:iter";

declare function values(): Iterator<&immutable int32>;
/// @type.symbol symbol=values source="declare function values(): Iterator<&immutable int32>" type=() => Iterator<&'managed immutable int32>
/// @resolution.name source=Iterator target=Iterator

const copied = values().cloned().toArray();
/// @type.symbol symbol=copied source=copied type=^int32[]
/// @resolution.pattern source=copied kind=binding target=copied
/// @resolution.name source=values target=values
/// @resolution.member source=values().cloned receiver=Iterator<&'managed immutable int32> type=<Iterator.cloned.U: Clone, Iterator.cloned.'a, const Iterator.cloned.A: "immutable" | "exclusive">(this: Iterator<&'managed immutable int32>) => CloneIterator<Iterator<&'managed immutable int32>, Iterator.cloned.U> kind=symbol target_receiver=Iterator<&'managed immutable int32> dispatch=dynamic constraint=Iterator<&'managed immutable int32> target=Iterator.cloned
/// @resolution.member source=values().cloned().toArray receiver=CloneIterator<Iterator<&'managed immutable int32>, int32> type=(this: CloneIterator<Iterator<&'managed immutable int32>, int32>) => ^^int32[] kind=symbol target_receiver=CloneIterator<Iterator<&'managed immutable int32>, int32> target=Iterator.toArray
/// @resolution.call source=values() parameters=() return=Iterator<&'managed immutable int32> kind=symbol target=values
/// @resolution.call source=values().cloned() parameters=() return=CloneIterator<Iterator<&'managed immutable int32>, int32> regions=("managed" & "local") kind=dynamic target=Iterator.cloned receiver=Iterator<&'managed immutable int32> constraint=Iterator<&'managed immutable int32> generic_arguments=(&'managed immutable int32, int32, "managed" & "local", "immutable")
/// @resolution.call source=values().cloned().toArray() parameters=() return=^^int32[] regions=("managed" & "local") kind=symbol target=Iterator.toArray receiver=CloneIterator<Iterator<&'managed immutable int32>, int32> instance="CloneIterator<Iterator<&'managed immutable int32>, int32>.<extension#19>.toArray"
/// @generic.instantiation id="Iterator.cloned<&'managed immutable int32>" template=Iterator.cloned arguments=(&'managed immutable int32)
/// @generic.instantiation id="Iterator.toArray<^int32, int32, \"managed\" & \"local\", \"immutable\", Iterator<&'managed immutable int32>>" template=Iterator.toArray arguments=(^int32, int32, "managed" & "local", "immutable", Iterator<&'managed immutable int32>)

copied satisfies int32[];
/// @resolution.name source=copied target=copied
/// @resolution.place source=copied placement="local" lifetime="static" access="immutable"
/// @resolution.access source=copied root=copied
"#,
        r#"
"#,
    );
}

/// Accumulate an array by spreading into a reduce callback.
#[test]
fn test_accumulate_an_array_by_spreading_into_a_reduce_callback() {
    let session = TestSession::single(
        r#"
function copy(source: int32[]): int32[] {
    return source.reduce((output, value) => [...output, value], []);
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
function copy(source: int32[]): int32[] {
    return source.reduce<int32, int32[], "managed">(
        (output: int32[], value: int32): int32[] => [...output, value],
        [],
    );
}

=== dir ===
function copy(source: int32[]): int32[] {
/// @type.symbol symbol=copy type=(int32[]) => int32[]
/// @type.symbol symbol=copy.source source="source: int32[]" type=int32[]

    return source.reduce((output, value) => [...output, value], []);
    /// @resolution.name source=source target=copy.source
    /// @resolution.member source=source.reduce receiver=int32[] type=<reduce.U#2, reduce#2.'a>(this: &reduce#2.'a readonly int32[], (reduce.U#2, int32, isize) => reduce.U#2, reduce.U#2) => reduce.U#2 kind=symbol target_receiver=int32[] target=reduce#2
    /// @resolution.call source="source.reduce((output, value) => [...output, value], [])" parameters=((int32[], int32, isize) => int32[], int32[]) arguments=(provided((output, value) => [...output, value]) as (int32[], int32, isize) => int32[], provided([]) as int32[]) return=int32[] regions=("managed" & "local") kind=symbol target=reduce#2 receiver=int32[] adjustments=(borrow(&'managed readonly int32[])) instance="Array<int32>.<extension#4>.reduce#2<int32[], \"managed\" & \"local\">"
    /// @resolution.place source=source placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=source root=copy.source
    /// @generic.instantiation id="reduce#2<int32, int32[], \"managed\" & \"local\">" template=reduce#2 arguments=(int32, int32[], "managed" & "local")
    /// @generic.instantiation id=reduce#2<int32> template=reduce#2 arguments=(int32)
    /// @type.symbol symbol=copy.symbol3 source="(output, value) => [...output, value]" type=Function<(int32[], int32), int32[], "readonly">
    /// @type.symbol symbol=copy.symbol3.output source=output type=int32[]
    /// @type.symbol symbol=copy.symbol3.value source=value type=int32
    /// @resolution.call source=[...output, value] parameters=(^Slice<int32>) arguments=(rest(spread(provided(...output) as int32[], iterator=iterator#2(parameters=(), arguments=(), return=Iterator<int32>), next=dynamic(Iterator<int32> as Iterator<int32>, Iterator.next)(parameters=(), arguments=(), return=IteratorResult<int32, void>, regions=("managed" & "local"))) as int32, provided(value) as int32) as int32) return=int32[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int32>
    /// @generic.instantiation id=iterator#2<int32> template=iterator#2 arguments=(int32)
    /// @resolution.name source=output target=copy.symbol3.output
    /// @resolution.place source=output placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=output root=copy.symbol3.output
    /// @resolution.name source=value target=copy.symbol3.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=copy.symbol3.value
    /// @resolution.call source=[] parameters=(^Slice<int32>) arguments=(rest() as int32) return=int32[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int32>
    /// @generic.instantiation id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)

}
"#,
        r#"
"#,
    );
}
