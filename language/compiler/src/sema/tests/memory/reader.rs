use crate::tests::{DirRows, TestSession};

#[test]
fn test_read_array_readers_through_every_receiver_form() {
    let session = TestSession::single(
        r#"
declare const owned: ^int32[];
declare const managed: int32[];
declare const borrowed: &int32[];
declare const view: &readonly int32[];

const ownedLength = owned.length;
const ownedEmpty = owned.isEmpty;
const ownedAt = owned.at(0);
const ownedIncludes = owned.includes(1);

const managedLength = managed.length;
const managedEmpty = managed.isEmpty;
const managedAt = managed.at(0);
const managedIncludes = managed.includes(1);

const borrowedLength = borrowed.length;
const borrowedEmpty = borrowed.isEmpty;
const borrowedAt = borrowed.at(0);
const borrowedIncludes = borrowed.includes(1);

const viewLength = view.length;
const viewEmpty = view.isEmpty;
const viewAt = view.at(0);
const viewIncludes = view.includes(1);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const owned: ^int32[];
declare const managed: int32[];
declare const borrowed: &'static int32[];
declare const view: &'static readonly int32[];

const ownedLength: isize = owned.length;
const ownedEmpty: boolean = owned.isEmpty;
const ownedAt: &'frame readonly int32 | undefined = owned.at<int32, "readonly">(0);
const ownedIncludes: boolean = owned.includes<int32, int32>(1 as &'frame readonly int32);

const managedLength: isize = managed.length;
const managedEmpty: boolean = managed.isEmpty;
const managedAt: int32 | undefined = managed.at<int32>(0);
const managedIncludes: boolean = managed.includes<int32, int32>(1 as &'frame readonly int32);

const borrowedLength: isize = borrowed.length;
const borrowedEmpty: boolean = borrowed.isEmpty;
const borrowedAt: &'static int32 | undefined = borrowed.at<int32, "mutable">(0);
const borrowedIncludes: boolean = borrowed.includes<int32, int32>(1 as &'frame readonly int32);

const viewLength: isize = view.length;
const viewEmpty: boolean = view.isEmpty;
const viewAt: &'static readonly int32 | undefined = view.at<int32, "readonly">(0);
const viewIncludes: boolean = view.includes<int32, int32>(1 as &'frame readonly int32);

=== dir ===
declare const owned: ^int32[];
/// @type.symbol symbol=owned source=owned type=Owned<int32[]>
/// @resolution.pattern source=owned kind=binding target=owned

declare const managed: int32[];
/// @type.symbol symbol=managed source=managed type=int32[]
/// @resolution.pattern source=managed kind=binding target=managed

declare const borrowed: &int32[];
/// @type.symbol symbol=borrowed source=borrowed type=&'static constant int32[]
/// @resolution.pattern source=borrowed kind=binding target=borrowed

declare const view: &readonly int32[];
/// @type.symbol symbol=view source=view type=&'static readonly constant int32[]
/// @resolution.pattern source=view kind=binding target=view

const ownedLength = owned.length;
/// @type.symbol symbol=ownedLength source=ownedLength type=isize
/// @resolution.pattern source=ownedLength kind=binding target=ownedLength
/// @resolution.name source=owned target=owned
/// @resolution.member source=owned.length receiver=Owned<int32[]> type=isize kind=call target="length(parameters=(), arguments=(), return=isize)"
/// @resolution.place source=owned placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=owned root=owned
/// @generic.instantiation id=length<int32> template=length arguments=(int32)

const ownedEmpty = owned.isEmpty;
/// @type.symbol symbol=ownedEmpty source=ownedEmpty type=boolean
/// @resolution.pattern source=ownedEmpty kind=binding target=ownedEmpty
/// @resolution.name source=owned target=owned
/// @resolution.member source=owned.isEmpty receiver=Owned<int32[]> type=boolean kind=call target="isEmpty(parameters=(), arguments=(), return=boolean)"
/// @resolution.place source=owned placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=owned root=owned
/// @generic.instantiation id=isEmpty<int32> template=isEmpty arguments=(int32)

const ownedAt = owned.at(0);
/// @type.symbol symbol=ownedAt source=ownedAt type=&'frame readonly int32 | undefined
/// @resolution.pattern source=ownedAt kind=binding target=ownedAt
/// @resolution.name source=owned target=owned
/// @resolution.member source=owned.at receiver=Owned<int32[]> type=(this: &'frame readonly int32[], isize) => &'frame readonly int32 | undefined kind=symbol target_receiver=Owned<int32[]> target=at#2
/// @resolution.call source=owned.at(0) parameters=(isize) arguments=(provided(0) as isize) return=&'frame readonly int32 | undefined kind=symbol target=at#2 receiver=Owned<int32[]> instance="Borrowed<T#4[], 'a, A#1>.<extension#4>.at#2"
/// @resolution.place source=owned placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=owned root=owned
/// @generic.instantiation id="at#2<int32, \"readonly\">" template=at#2 arguments=(int32, "readonly")

const ownedIncludes = owned.includes(1);
/// @type.symbol symbol=ownedIncludes source=ownedIncludes type=boolean
/// @resolution.pattern source=ownedIncludes kind=binding target=ownedIncludes
/// @resolution.name source=owned target=owned
/// @resolution.member source=owned.includes receiver=Owned<int32[]> type=<includes.Q, includes.'a, includes.'b>(this: &includes.'a readonly int32[], &includes.'b readonly includes.Q, isize | undefined?) => boolean kind=symbol target_receiver=Owned<int32[]> target=includes
/// @resolution.call source=owned.includes(1) parameters=(&'frame readonly int32, isize | undefined) arguments=(provided(1) as &'frame readonly int32, omitted as isize | undefined) return=boolean kind=symbol target=includes receiver=Owned<int32[]> adjustments=(borrow(&'static readonly constant Owned<int32[]>)) instance=Array<int32>.<extension#5>.includes<int32>
/// @resolution.place source=owned placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=owned root=owned
/// @generic.instantiation id="includes<int32, int32>" template=includes arguments=(int32, int32)
/// @generic.instantiation id=includes<int32> template=includes arguments=(int32)

const managedLength = managed.length;
/// @type.symbol symbol=managedLength source=managedLength type=isize
/// @resolution.pattern source=managedLength kind=binding target=managedLength
/// @resolution.name source=managed target=managed
/// @resolution.member source=managed.length receiver=int32[] type=isize kind=call target="length(parameters=(), arguments=(), return=isize)"
/// @resolution.place source=managed placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=managed root=managed

const managedEmpty = managed.isEmpty;
/// @type.symbol symbol=managedEmpty source=managedEmpty type=boolean
/// @resolution.pattern source=managedEmpty kind=binding target=managedEmpty
/// @resolution.name source=managed target=managed
/// @resolution.member source=managed.isEmpty receiver=int32[] type=boolean kind=call target="isEmpty(parameters=(), arguments=(), return=boolean)"
/// @resolution.place source=managed placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=managed root=managed

const managedAt = managed.at(0);
/// @type.symbol symbol=managedAt source=managedAt type=int32 | undefined
/// @resolution.pattern source=managedAt kind=binding target=managedAt
/// @resolution.name source=managed target=managed
/// @resolution.member source=managed.at receiver=int32[] type=<at#1.P0: Place>(this: Managed<int32[], at#1.P0>, isize) => int32 | undefined kind=symbol target_receiver=int32[] target=at#1
/// @resolution.call source=managed.at(0) parameters=(isize) arguments=(provided(0) as isize) return=int32 | undefined kind=symbol target=at#1 receiver=int32[] instance="Array<int32>.<extension#3>.at#1<\"local\">"
/// @resolution.place source=managed placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=managed root=managed
/// @generic.instantiation id="at#1<int32, \"local\">" template=at#1 arguments=(int32, "local")
/// @generic.instantiation id=at#1<int32> template=at#1 arguments=(int32)

const managedIncludes = managed.includes(1);
/// @type.symbol symbol=managedIncludes source=managedIncludes type=boolean
/// @resolution.pattern source=managedIncludes kind=binding target=managedIncludes
/// @resolution.name source=managed target=managed
/// @resolution.member source=managed.includes receiver=int32[] type=<includes.Q, includes.'a, includes.'b>(this: &includes.'a readonly int32[], &includes.'b readonly includes.Q, isize | undefined?) => boolean kind=symbol target_receiver=int32[] target=includes
/// @resolution.call source=managed.includes(1) parameters=(&'frame readonly int32, isize | undefined) arguments=(provided(1) as &'frame readonly int32, omitted as isize | undefined) return=boolean kind=symbol target=includes receiver=int32[] adjustments=(borrow(&'static readonly int32[])) instance=Array<int32>.<extension#5>.includes<int32>
/// @resolution.place source=managed placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=managed root=managed

const borrowedLength = borrowed.length;
/// @type.symbol symbol=borrowedLength source=borrowedLength type=isize
/// @resolution.pattern source=borrowedLength kind=binding target=borrowedLength
/// @resolution.name source=borrowed target=borrowed
/// @resolution.member source=borrowed.length receiver=&'static constant int32[] type=isize kind=call target="length(parameters=(), arguments=(), return=isize)"
/// @resolution.place source=borrowed placement="constant" lifetime="static" access="mutable"
/// @resolution.access source=borrowed root=borrowed

const borrowedEmpty = borrowed.isEmpty;
/// @type.symbol symbol=borrowedEmpty source=borrowedEmpty type=boolean
/// @resolution.pattern source=borrowedEmpty kind=binding target=borrowedEmpty
/// @resolution.name source=borrowed target=borrowed
/// @resolution.member source=borrowed.isEmpty receiver=&'static constant int32[] type=boolean kind=call target="isEmpty(parameters=(), arguments=(), return=boolean)"
/// @resolution.place source=borrowed placement="constant" lifetime="static" access="mutable"
/// @resolution.access source=borrowed root=borrowed

const borrowedAt = borrowed.at(0);
/// @type.symbol symbol=borrowedAt source=borrowedAt type=&'static constant int32 | undefined
/// @resolution.pattern source=borrowedAt kind=binding target=borrowedAt
/// @resolution.name source=borrowed target=borrowed
/// @resolution.member source=borrowed.at receiver=&'static constant int32[] type=(this: &'static constant int32[], isize) => &'static constant int32 | undefined kind=symbol target_receiver=&'static constant int32[] target=at#2
/// @resolution.call source=borrowed.at(0) parameters=(isize) arguments=(provided(0) as isize) return=&'static constant int32 | undefined kind=symbol target=at#2 receiver=&'static constant int32[] instance="Borrowed<T#4[], 'a, A#1>.<extension#4>.at#2"
/// @resolution.place source=borrowed placement="constant" lifetime="static" access="mutable"
/// @resolution.access source=borrowed root=borrowed
/// @generic.instantiation id="at#2<int32, \"mutable\">" template=at#2 arguments=(int32, "mutable")

const borrowedIncludes = borrowed.includes(1);
/// @type.symbol symbol=borrowedIncludes source=borrowedIncludes type=boolean
/// @resolution.pattern source=borrowedIncludes kind=binding target=borrowedIncludes
/// @resolution.name source=borrowed target=borrowed
/// @resolution.member source=borrowed.includes receiver=&'static constant int32[] type=<includes.Q, includes.'a, includes.'b>(this: &includes.'a readonly int32[], &includes.'b readonly includes.Q, isize | undefined?) => boolean kind=symbol target_receiver=&'static constant int32[] target=includes
/// @resolution.call source=borrowed.includes(1) parameters=(&'frame readonly int32, isize | undefined) arguments=(provided(1) as &'frame readonly int32, omitted as isize | undefined) return=boolean kind=symbol target=includes receiver=&'static constant int32[] instance=Array<int32>.<extension#5>.includes<int32>
/// @resolution.place source=borrowed placement="constant" lifetime="static" access="mutable"
/// @resolution.access source=borrowed root=borrowed

const viewLength = view.length;
/// @type.symbol symbol=viewLength source=viewLength type=isize
/// @resolution.pattern source=viewLength kind=binding target=viewLength
/// @resolution.name source=view target=view
/// @resolution.member source=view.length receiver=&'static readonly constant int32[] type=isize kind=call target="length(parameters=(), arguments=(), return=isize)"
/// @resolution.place source=view placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=view root=view

const viewEmpty = view.isEmpty;
/// @type.symbol symbol=viewEmpty source=viewEmpty type=boolean
/// @resolution.pattern source=viewEmpty kind=binding target=viewEmpty
/// @resolution.name source=view target=view
/// @resolution.member source=view.isEmpty receiver=&'static readonly constant int32[] type=boolean kind=call target="isEmpty(parameters=(), arguments=(), return=boolean)"
/// @resolution.place source=view placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=view root=view

const viewAt = view.at(0);
/// @type.symbol symbol=viewAt source=viewAt type=&'static readonly constant int32 | undefined
/// @resolution.pattern source=viewAt kind=binding target=viewAt
/// @resolution.name source=view target=view
/// @resolution.member source=view.at receiver=&'static readonly constant int32[] type=(this: &'static readonly constant int32[], isize) => &'static readonly constant int32 | undefined kind=symbol target_receiver=&'static readonly constant int32[] target=at#2
/// @resolution.call source=view.at(0) parameters=(isize) arguments=(provided(0) as isize) return=&'static readonly constant int32 | undefined kind=symbol target=at#2 receiver=&'static readonly constant int32[] instance="Borrowed<T#4[], 'a, A#1>.<extension#4>.at#2"
/// @resolution.place source=view placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=view root=view

const viewIncludes = view.includes(1);
/// @type.symbol symbol=viewIncludes source=viewIncludes type=boolean
/// @resolution.pattern source=viewIncludes kind=binding target=viewIncludes
/// @resolution.name source=view target=view
/// @resolution.member source=view.includes receiver=&'static readonly constant int32[] type=<includes.Q, includes.'a, includes.'b>(this: &includes.'a readonly int32[], &includes.'b readonly includes.Q, isize | undefined?) => boolean kind=symbol target_receiver=&'static readonly constant int32[] target=includes
/// @resolution.call source=view.includes(1) parameters=(&'frame readonly int32, isize | undefined) arguments=(provided(1) as &'frame readonly int32, omitted as isize | undefined) return=boolean kind=symbol target=includes receiver=&'static readonly constant int32[] instance=Array<int32>.<extension#5>.includes<int32>
/// @resolution.place source=view placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=view root=view
"#,
        r#"
/// @diagnostic.error id=receiver-not-assignable message="receiver type '^int32[]' is not assignable to the method's 'this' type '&readonly local int32[]'"
/// @diagnostic.label line=9 column=17 span="owned.at(0)" line_source="const ownedAt = owned.at(0);"
"#,
    );
}

#[test]
fn test_read_string_readers_through_every_receiver_form() {
    let session = TestSession::single(
        r#"
declare const owned: ^string;
declare const managed: string;
declare const view: &readonly string;

const ownedLength = owned.length;
const ownedEmpty = owned.isEmpty;
const ownedIncludes = owned.includes("a");
const ownedTrimmed = owned.trim();

const managedLength = managed.length;
const managedEmpty = managed.isEmpty;
const managedIncludes = managed.includes("a");
const managedTrimmed = managed.trim();

const viewLength = view.length;
const viewEmpty = view.isEmpty;
const viewIncludes = view.includes("a");
const viewTrimmed = view.trim();
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const owned: ^string;
declare const managed: string;
declare const view: &'static readonly string;

const ownedLength: isize = owned.length;
const ownedEmpty: boolean = owned.isEmpty;
const ownedIncludes: boolean = owned.includes("a" as &'frame readonly string);
const ownedTrimmed: string = owned.trim() as string;

const managedLength: isize = managed.length;
const managedEmpty: boolean = managed.isEmpty;
const managedIncludes: boolean = managed.includes("a" as &'frame readonly string);
const managedTrimmed: string = managed.trim() as string;

const viewLength: isize = view.length;
const viewEmpty: boolean = view.isEmpty;
const viewIncludes: boolean = view.includes("a" as &'frame readonly string);
const viewTrimmed: string = view.trim() as string;

=== dir ===
declare const owned: ^string;
/// @type.symbol symbol=owned source=owned type=Owned<string>
/// @resolution.pattern source=owned kind=binding target=owned

declare const managed: string;
/// @type.symbol symbol=managed source=managed type=string
/// @resolution.pattern source=managed kind=binding target=managed

declare const view: &readonly string;
/// @type.symbol symbol=view source=view type=&'static readonly constant string
/// @resolution.pattern source=view kind=binding target=view

const ownedLength = owned.length;
/// @type.symbol symbol=ownedLength source=ownedLength type=isize
/// @resolution.pattern source=ownedLength kind=binding target=ownedLength
/// @resolution.name source=owned target=owned
/// @resolution.member source=owned.length receiver=Owned<string> type=isize kind=call target="length(parameters=(), arguments=(), return=isize)"
/// @resolution.place source=owned placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=owned root=owned

const ownedEmpty = owned.isEmpty;
/// @type.symbol symbol=ownedEmpty source=ownedEmpty type=boolean
/// @resolution.pattern source=ownedEmpty kind=binding target=ownedEmpty
/// @resolution.name source=owned target=owned
/// @resolution.member source=owned.isEmpty receiver=Owned<string> type=boolean kind=call target="isEmpty(parameters=(), arguments=(), return=boolean)"
/// @resolution.place source=owned placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=owned root=owned

const ownedIncludes = owned.includes("a");
/// @type.symbol symbol=ownedIncludes source=ownedIncludes type=boolean
/// @resolution.pattern source=ownedIncludes kind=binding target=ownedIncludes
/// @resolution.name source=owned target=owned
/// @resolution.member source=owned.includes receiver=Owned<string> type=<includes.'a, includes.'b>(this: &includes.'a readonly string, &includes.'b readonly string, isize | undefined?) => boolean kind=symbol target_receiver=Owned<string> target=includes
/// @resolution.call source="owned.includes(\"a\")" parameters=(&'frame readonly string, isize | undefined) arguments=(provided("a") as &'frame readonly string, omitted as isize | undefined) return=boolean kind=symbol target=includes receiver=Owned<string> adjustments=(borrow(&'static readonly constant Owned<string>))
/// @resolution.place source=owned placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=owned root=owned

const ownedTrimmed = owned.trim();
/// @type.symbol symbol=ownedTrimmed source=ownedTrimmed type=string
/// @resolution.pattern source=ownedTrimmed kind=binding target=ownedTrimmed
/// @resolution.name source=owned target=owned
/// @resolution.member source=owned.trim receiver=Owned<string> type=<trim.'a>(this: &trim.'a readonly string) => Owned<string> kind=symbol target_receiver=Owned<string> target=trim
/// @resolution.call source=owned.trim() parameters=() return=Owned<string> kind=symbol target=trim receiver=Owned<string> adjustments=(borrow(&'static readonly constant Owned<string>))
/// @resolution.place source=owned placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=owned root=owned

const managedLength = managed.length;
/// @type.symbol symbol=managedLength source=managedLength type=isize
/// @resolution.pattern source=managedLength kind=binding target=managedLength
/// @resolution.name source=managed target=managed
/// @resolution.member source=managed.length receiver=string type=isize kind=call target="length(parameters=(), arguments=(), return=isize)"
/// @resolution.place source=managed placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=managed root=managed

const managedEmpty = managed.isEmpty;
/// @type.symbol symbol=managedEmpty source=managedEmpty type=boolean
/// @resolution.pattern source=managedEmpty kind=binding target=managedEmpty
/// @resolution.name source=managed target=managed
/// @resolution.member source=managed.isEmpty receiver=string type=boolean kind=call target="isEmpty(parameters=(), arguments=(), return=boolean)"
/// @resolution.place source=managed placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=managed root=managed

const managedIncludes = managed.includes("a");
/// @type.symbol symbol=managedIncludes source=managedIncludes type=boolean
/// @resolution.pattern source=managedIncludes kind=binding target=managedIncludes
/// @resolution.name source=managed target=managed
/// @resolution.member source=managed.includes receiver=string type=<includes.'a, includes.'b>(this: &includes.'a readonly string, &includes.'b readonly string, isize | undefined?) => boolean kind=symbol target_receiver=string target=includes
/// @resolution.call source="managed.includes(\"a\")" parameters=(&'frame readonly string, isize | undefined) arguments=(provided("a") as &'frame readonly string, omitted as isize | undefined) return=boolean kind=symbol target=includes receiver=string adjustments=(borrow(&'static readonly string))
/// @resolution.place source=managed placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=managed root=managed

const managedTrimmed = managed.trim();
/// @type.symbol symbol=managedTrimmed source=managedTrimmed type=string
/// @resolution.pattern source=managedTrimmed kind=binding target=managedTrimmed
/// @resolution.name source=managed target=managed
/// @resolution.member source=managed.trim receiver=string type=<trim.'a>(this: &trim.'a readonly string) => Owned<string> kind=symbol target_receiver=string target=trim
/// @resolution.call source=managed.trim() parameters=() return=Owned<string> kind=symbol target=trim receiver=string adjustments=(borrow(&'static readonly string))
/// @resolution.place source=managed placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=managed root=managed

const viewLength = view.length;
/// @type.symbol symbol=viewLength source=viewLength type=isize
/// @resolution.pattern source=viewLength kind=binding target=viewLength
/// @resolution.name source=view target=view
/// @resolution.member source=view.length receiver=&'static readonly constant string type=isize kind=call target="length(parameters=(), arguments=(), return=isize)"
/// @resolution.place source=view placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=view root=view

const viewEmpty = view.isEmpty;
/// @type.symbol symbol=viewEmpty source=viewEmpty type=boolean
/// @resolution.pattern source=viewEmpty kind=binding target=viewEmpty
/// @resolution.name source=view target=view
/// @resolution.member source=view.isEmpty receiver=&'static readonly constant string type=boolean kind=call target="isEmpty(parameters=(), arguments=(), return=boolean)"
/// @resolution.place source=view placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=view root=view

const viewIncludes = view.includes("a");
/// @type.symbol symbol=viewIncludes source=viewIncludes type=boolean
/// @resolution.pattern source=viewIncludes kind=binding target=viewIncludes
/// @resolution.name source=view target=view
/// @resolution.member source=view.includes receiver=&'static readonly constant string type=<includes.'a, includes.'b>(this: &includes.'a readonly string, &includes.'b readonly string, isize | undefined?) => boolean kind=symbol target_receiver=&'static readonly constant string target=includes
/// @resolution.call source="view.includes(\"a\")" parameters=(&'frame readonly string, isize | undefined) arguments=(provided("a") as &'frame readonly string, omitted as isize | undefined) return=boolean kind=symbol target=includes receiver=&'static readonly constant string
/// @resolution.place source=view placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=view root=view

const viewTrimmed = view.trim();
/// @type.symbol symbol=viewTrimmed source=viewTrimmed type=string
/// @resolution.pattern source=viewTrimmed kind=binding target=viewTrimmed
/// @resolution.name source=view target=view
/// @resolution.member source=view.trim receiver=&'static readonly constant string type=<trim.'a>(this: &trim.'a readonly string) => Owned<string> kind=symbol target_receiver=&'static readonly constant string target=trim
/// @resolution.call source=view.trim() parameters=() return=Owned<string> kind=symbol target=trim receiver=&'static readonly constant string
/// @resolution.place source=view placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=view root=view
"#,
        r#"

"#,
    );
}

#[test]
fn test_read_map_and_set_readers_through_every_receiver_form() {
    let session = TestSession::single(
        r#"
import { Map, Set } from "destack:collections";

declare const ownedMap: ^Map<string, int32>;
declare const managedMap: Map<string, int32>;
declare const viewMap: &readonly Map<string, int32>;
declare const ownedSet: ^Set<int32>;
declare const managedSet: Set<int32>;
declare const viewSet: &readonly Set<int32>;

const ownedMapSize = ownedMap.size;
const ownedMapHas = ownedMap.has("a");
const managedMapSize = managedMap.size;
const managedMapHas = managedMap.has("a");
const viewMapSize = viewMap.size;
const viewMapHas = viewMap.has("a");

const ownedSetSize = ownedSet.size;
const ownedSetHas = ownedSet.has(1);
const managedSetSize = managedSet.size;
const managedSetHas = managedSet.has(1);
const viewSetSize = viewSet.size;
const viewSetHas = viewSet.has(1);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Map, Set } from "destack:collections";

declare const ownedMap: ^Map<string, int32>;
declare const managedMap: Map<string, int32>;
declare const viewMap: &'static readonly Map<string, int32>;
declare const ownedSet: ^Set<int32>;
declare const managedSet: Set<int32>;
declare const viewSet: &'static readonly Set<int32>;

const ownedMapSize: usize = ownedMap.size;
const ownedMapHas: boolean = ownedMap.has<string, int32, string>("a" as &'frame readonly string);
const managedMapSize: usize = managedMap.size;
const managedMapHas: boolean = managedMap.has<string, int32, string>(
    "a" as &'frame readonly string,
);
const viewMapSize: usize = viewMap.size;
const viewMapHas: boolean = viewMap.has<string, int32, string>("a" as &'frame readonly string);

const ownedSetSize: usize = ownedSet.size;
const ownedSetHas: boolean = ownedSet.has<int32, int32>(1 as &'frame readonly int32);
const managedSetSize: usize = managedSet.size;
const managedSetHas: boolean = managedSet.has<int32, int32>(1 as &'frame readonly int32);
const viewSetSize: usize = viewSet.size;
const viewSetHas: boolean = viewSet.has<int32, int32>(1 as &'frame readonly int32);

=== dir ===
import { Map, Set } from "destack:collections";

declare const ownedMap: ^Map<string, int32>;
/// @type.symbol symbol=ownedMap source=ownedMap type=Owned<Map<string, int32>>
/// @resolution.pattern source=ownedMap kind=binding target=ownedMap
/// @resolution.name source=Map target=Map

declare const managedMap: Map<string, int32>;
/// @type.symbol symbol=managedMap source=managedMap type=Map<string, int32>
/// @resolution.pattern source=managedMap kind=binding target=managedMap
/// @resolution.name source=Map target=Map

declare const viewMap: &readonly Map<string, int32>;
/// @type.symbol symbol=viewMap source=viewMap type=&'static readonly constant Map<string, int32>
/// @resolution.pattern source=viewMap kind=binding target=viewMap
/// @resolution.name source=Map target=Map

declare const ownedSet: ^Set<int32>;
/// @type.symbol symbol=ownedSet source=ownedSet type=Owned<Set<int32>>
/// @resolution.pattern source=ownedSet kind=binding target=ownedSet
/// @resolution.name source=Set target=Set

declare const managedSet: Set<int32>;
/// @type.symbol symbol=managedSet source=managedSet type=Set<int32>
/// @resolution.pattern source=managedSet kind=binding target=managedSet
/// @resolution.name source=Set target=Set

declare const viewSet: &readonly Set<int32>;
/// @type.symbol symbol=viewSet source=viewSet type=&'static readonly constant Set<int32>
/// @resolution.pattern source=viewSet kind=binding target=viewSet
/// @resolution.name source=Set target=Set

const ownedMapSize = ownedMap.size;
/// @type.symbol symbol=ownedMapSize source=ownedMapSize type=usize
/// @resolution.pattern source=ownedMapSize kind=binding target=ownedMapSize
/// @resolution.name source=ownedMap target=ownedMap
/// @resolution.member source=ownedMap.size receiver=Owned<Map<string, int32>> type=usize kind=call target="size(parameters=(), arguments=(), return=usize)"
/// @resolution.place source=ownedMap placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=ownedMap root=ownedMap
/// @generic.instantiation id="size<string, int32>" template=size arguments=(string, int32)

const ownedMapHas = ownedMap.has("a");
/// @type.symbol symbol=ownedMapHas source=ownedMapHas type=boolean
/// @resolution.pattern source=ownedMapHas kind=binding target=ownedMapHas
/// @resolution.name source=ownedMap target=ownedMap
/// @resolution.member source=ownedMap.has receiver=Owned<Map<string, int32>> type=<has.Q: Hash, has.'a, has.'b>(this: &has.'a readonly Map<string, int32>, &has.'b readonly has.Q) => boolean kind=symbol target_receiver=Owned<Map<string, int32>> target=has
/// @resolution.call source="ownedMap.has(\"a\")" parameters=(&'frame readonly string) arguments=(provided("a") as &'frame readonly string) return=boolean kind=symbol target=has receiver=Owned<Map<string, int32>> adjustments=(borrow(&'static readonly constant Owned<Map<string, int32>>)) instance="Map<string, int32>.<extension#8>.has<string>"
/// @resolution.place source=ownedMap placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=ownedMap root=ownedMap
/// @generic.instantiation id="has<string, int32, string>" template=has arguments=(string, int32, string)
/// @generic.instantiation id="has<string, int32>" template=has arguments=(string, int32)

const managedMapSize = managedMap.size;
/// @type.symbol symbol=managedMapSize source=managedMapSize type=usize
/// @resolution.pattern source=managedMapSize kind=binding target=managedMapSize
/// @resolution.name source=managedMap target=managedMap
/// @resolution.member source=managedMap.size receiver=Map<string, int32> type=usize kind=call target="size(parameters=(), arguments=(), return=usize)"
/// @resolution.place source=managedMap placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=managedMap root=managedMap

const managedMapHas = managedMap.has("a");
/// @type.symbol symbol=managedMapHas source=managedMapHas type=boolean
/// @resolution.pattern source=managedMapHas kind=binding target=managedMapHas
/// @resolution.name source=managedMap target=managedMap
/// @resolution.member source=managedMap.has receiver=Map<string, int32> type=<has.Q: Hash, has.'a, has.'b>(this: &has.'a readonly Map<string, int32>, &has.'b readonly has.Q) => boolean kind=symbol target_receiver=Map<string, int32> target=has
/// @resolution.call source="managedMap.has(\"a\")" parameters=(&'frame readonly string) arguments=(provided("a") as &'frame readonly string) return=boolean kind=symbol target=has receiver=Map<string, int32> adjustments=(borrow(&'static readonly Map<string, int32>)) instance="Map<string, int32>.<extension#8>.has<string>"
/// @resolution.place source=managedMap placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=managedMap root=managedMap

const viewMapSize = viewMap.size;
/// @type.symbol symbol=viewMapSize source=viewMapSize type=usize
/// @resolution.pattern source=viewMapSize kind=binding target=viewMapSize
/// @resolution.name source=viewMap target=viewMap
/// @resolution.member source=viewMap.size receiver=&'static readonly constant Map<string, int32> type=usize kind=call target="size(parameters=(), arguments=(), return=usize)"
/// @resolution.place source=viewMap placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=viewMap root=viewMap

const viewMapHas = viewMap.has("a");
/// @type.symbol symbol=viewMapHas source=viewMapHas type=boolean
/// @resolution.pattern source=viewMapHas kind=binding target=viewMapHas
/// @resolution.name source=viewMap target=viewMap
/// @resolution.member source=viewMap.has receiver=&'static readonly constant Map<string, int32> type=<has.Q: Hash, has.'a, has.'b>(this: &has.'a readonly Map<string, int32>, &has.'b readonly has.Q) => boolean kind=symbol target_receiver=&'static readonly constant Map<string, int32> target=has
/// @resolution.call source="viewMap.has(\"a\")" parameters=(&'frame readonly string) arguments=(provided("a") as &'frame readonly string) return=boolean kind=symbol target=has receiver=&'static readonly constant Map<string, int32> instance="Map<string, int32>.<extension#8>.has<string>"
/// @resolution.place source=viewMap placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=viewMap root=viewMap

const ownedSetSize = ownedSet.size;
/// @type.symbol symbol=ownedSetSize source=ownedSetSize type=usize
/// @resolution.pattern source=ownedSetSize kind=binding target=ownedSetSize
/// @resolution.name source=ownedSet target=ownedSet
/// @resolution.member source=ownedSet.size receiver=Owned<Set<int32>> type=usize kind=call target="size(parameters=(), arguments=(), return=usize)"
/// @resolution.place source=ownedSet placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=ownedSet root=ownedSet
/// @generic.instantiation id=size<int32> template=size arguments=(int32)

const ownedSetHas = ownedSet.has(1);
/// @type.symbol symbol=ownedSetHas source=ownedSetHas type=boolean
/// @resolution.pattern source=ownedSetHas kind=binding target=ownedSetHas
/// @resolution.name source=ownedSet target=ownedSet
/// @resolution.member source=ownedSet.has receiver=Owned<Set<int32>> type=<has.Q: Hash, has.'a, has.'b>(this: &has.'a readonly Set<int32>, &has.'b readonly has.Q) => boolean kind=symbol target_receiver=Owned<Set<int32>> target=has
/// @resolution.call source=ownedSet.has(1) parameters=(&'frame readonly int32) arguments=(provided(1) as &'frame readonly int32) return=boolean kind=symbol target=has receiver=Owned<Set<int32>> adjustments=(borrow(&'static readonly constant Owned<Set<int32>>)) instance=Set<int32>.<extension#5>.has<int32>
/// @resolution.place source=ownedSet placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=ownedSet root=ownedSet
/// @generic.instantiation id="has<int32, int32>" template=has arguments=(int32, int32)
/// @generic.instantiation id=has<int32> template=has arguments=(int32)

const managedSetSize = managedSet.size;
/// @type.symbol symbol=managedSetSize source=managedSetSize type=usize
/// @resolution.pattern source=managedSetSize kind=binding target=managedSetSize
/// @resolution.name source=managedSet target=managedSet
/// @resolution.member source=managedSet.size receiver=Set<int32> type=usize kind=call target="size(parameters=(), arguments=(), return=usize)"
/// @resolution.place source=managedSet placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=managedSet root=managedSet

const managedSetHas = managedSet.has(1);
/// @type.symbol symbol=managedSetHas source=managedSetHas type=boolean
/// @resolution.pattern source=managedSetHas kind=binding target=managedSetHas
/// @resolution.name source=managedSet target=managedSet
/// @resolution.member source=managedSet.has receiver=Set<int32> type=<has.Q: Hash, has.'a, has.'b>(this: &has.'a readonly Set<int32>, &has.'b readonly has.Q) => boolean kind=symbol target_receiver=Set<int32> target=has
/// @resolution.call source=managedSet.has(1) parameters=(&'frame readonly int32) arguments=(provided(1) as &'frame readonly int32) return=boolean kind=symbol target=has receiver=Set<int32> adjustments=(borrow(&'static readonly Set<int32>)) instance=Set<int32>.<extension#5>.has<int32>
/// @resolution.place source=managedSet placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=managedSet root=managedSet

const viewSetSize = viewSet.size;
/// @type.symbol symbol=viewSetSize source=viewSetSize type=usize
/// @resolution.pattern source=viewSetSize kind=binding target=viewSetSize
/// @resolution.name source=viewSet target=viewSet
/// @resolution.member source=viewSet.size receiver=&'static readonly constant Set<int32> type=usize kind=call target="size(parameters=(), arguments=(), return=usize)"
/// @resolution.place source=viewSet placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=viewSet root=viewSet

const viewSetHas = viewSet.has(1);
/// @type.symbol symbol=viewSetHas source=viewSetHas type=boolean
/// @resolution.pattern source=viewSetHas kind=binding target=viewSetHas
/// @resolution.name source=viewSet target=viewSet
/// @resolution.member source=viewSet.has receiver=&'static readonly constant Set<int32> type=<has.Q: Hash, has.'a, has.'b>(this: &has.'a readonly Set<int32>, &has.'b readonly has.Q) => boolean kind=symbol target_receiver=&'static readonly constant Set<int32> target=has
/// @resolution.call source=viewSet.has(1) parameters=(&'frame readonly int32) arguments=(provided(1) as &'frame readonly int32) return=boolean kind=symbol target=has receiver=&'static readonly constant Set<int32> instance=Set<int32>.<extension#5>.has<int32>
/// @resolution.place source=viewSet placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=viewSet root=viewSet
"#,
        r#"

"#,
    );
}

#[test]
fn test_read_a_readonly_this_getter_through_every_receiver_form() {
    let session = TestSession::single(
        r#"
class Counter {
    count: int32 = 0;

    get total(&readonly this): int32 {
        return this.count;
    }

    bump(&exclusive this): void {
        this.count += 1;
    }
}

declare const owned: ^Counter;
declare const managed: Counter;
declare const borrowed: &Counter;
declare const view: &readonly Counter;

const ownedTotal = owned.total;
const managedTotal = managed.total;
const borrowedTotal = borrowed.total;
const viewTotal = view.total;

function mutate(owned: ^Counter, managed: Counter, borrowed: &Counter, view: &readonly Counter): void {
    owned.bump();
    managed.bump();
    borrowed.bump();
    view.bump();
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Counter {
    count: int32 = 0;

    get total(&readonly this): int32 {
        return this.count;
    }

    bump(&exclusive this): void {
        this.count += 1;
    }
}

declare const owned: ^Counter;
declare const managed: Counter;
declare const borrowed: &'static Counter;
declare const view: &'static readonly Counter;

const ownedTotal: int32 = owned.total;
const managedTotal: int32 = managed.total;
const borrowedTotal: int32 = borrowed.total;
const viewTotal: int32 = view.total;

function mutate<'a, 'b>(
    owned: ^Counter,
    managed: Counter,
    borrowed: &'a Counter,
    view: &'b readonly Counter,
): void {
    owned.bump();
    managed.bump();
    borrowed.bump();
    view.bump();
}

=== dir ===
class Counter {
/// @type.symbol symbol=Counter type=Counter
/// @definition.class symbol=Counter
/// @definition.field symbol=Counter.count source="count: int32 = 0" key=count type=int32
/// @definition.method symbol=Counter.bump slot=bump type=<Counter.bump.'a>(this: &Counter.bump.'a exclusive this) => void
/// @definition.method symbol=Counter.total slot=total role=getter type=<Counter.total.'a>(this: &Counter.total.'a readonly this) => int32

    count: int32 = 0;
    /// @type.symbol symbol=Counter.count source="count: int32 = 0" type=int32

    get total(&readonly this): int32 {
    /// @generic.template symbol=Counter.total parameters=('a)
    /// @type.symbol symbol=Counter.total type=<Counter.total.'a>(this: &Counter.total.'a readonly this) => int32
    /// @type.symbol symbol=Counter.total.this source="&readonly this" type=&Counter.total.'a readonly this

        return this.count;
        /// @resolution.member source=this.count receiver=&Counter.total.'a readonly Counter type=int32 kind=field target_receiver=&Counter.total.'a readonly Counter key=count target=Counter.count target_type=int32
        /// @resolution.receiver source=this kind=this declaration=Counter type=&Counter.total.'a readonly Counter
        /// @resolution.place source=this placement=Counter.total.'a lifetime=Counter.total.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.count placement=Counter.total.'a lifetime=Counter.total.'a access="readonly"
        /// @resolution.access source=this.count root=this keys=[count]

    }

    bump(&exclusive this): void {
    /// @generic.template symbol=Counter.bump parameters=('a)
    /// @type.symbol symbol=Counter.bump type=<Counter.bump.'a>(this: &Counter.bump.'a exclusive this) => void
    /// @type.symbol symbol=Counter.bump.this source="&exclusive this" type=&Counter.bump.'a exclusive this

        this.count += 1;
        /// @resolution.operator source="this.count += 1" type=int32 operator="+" kind=builtin operands=[this.count as int32 families=(integer), 1 as int32 families=(integer)]
        /// @resolution.receiver source=this kind=this declaration=Counter type=&Counter.bump.'a exclusive Counter
        /// @resolution.place source=this placement=Counter.bump.'a lifetime=Counter.bump.'a access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.count kind=place
        /// @resolution.assignment source=this.count read="receiver=&Counter.bump.'a exclusive Counter, target=field(receiver=&Counter.bump.'a exclusive Counter, target=Counter.count, type=int32), type=int32" write="receiver=&Counter.bump.'a exclusive Counter, target=field(receiver=&Counter.bump.'a exclusive Counter, target=Counter.count, type=int32), type=int32" type=int32
        /// @resolution.access source=this.count root=this keys=[count]

    }
}

declare const owned: ^Counter;
/// @type.symbol symbol=owned source=owned type=Owned<Counter>
/// @resolution.pattern source=owned kind=binding target=owned
/// @resolution.name source=Counter target=Counter

declare const managed: Counter;
/// @type.symbol symbol=managed source=managed type=Counter
/// @resolution.pattern source=managed kind=binding target=managed
/// @resolution.name source=Counter target=Counter

declare const borrowed: &Counter;
/// @type.symbol symbol=borrowed source=borrowed type=&'static constant Counter
/// @resolution.pattern source=borrowed kind=binding target=borrowed
/// @resolution.name source=Counter target=Counter

declare const view: &readonly Counter;
/// @type.symbol symbol=view source=view type=&'static readonly constant Counter
/// @resolution.pattern source=view kind=binding target=view
/// @resolution.name source=Counter target=Counter

const ownedTotal = owned.total;
/// @type.symbol symbol=ownedTotal source=ownedTotal type=int32
/// @resolution.pattern source=ownedTotal kind=binding target=ownedTotal
/// @resolution.name source=owned target=owned
/// @resolution.member source=owned.total receiver=Owned<Counter> type=int32 kind=call target="Counter.total(parameters=(), arguments=(), return=int32)"
/// @resolution.place source=owned placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=owned root=owned

const managedTotal = managed.total;
/// @type.symbol symbol=managedTotal source=managedTotal type=int32
/// @resolution.pattern source=managedTotal kind=binding target=managedTotal
/// @resolution.name source=managed target=managed
/// @resolution.member source=managed.total receiver=Counter type=int32 kind=call target="Counter.total(parameters=(), arguments=(), return=int32)"
/// @resolution.place source=managed placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=managed root=managed

const borrowedTotal = borrowed.total;
/// @type.symbol symbol=borrowedTotal source=borrowedTotal type=int32
/// @resolution.pattern source=borrowedTotal kind=binding target=borrowedTotal
/// @resolution.name source=borrowed target=borrowed
/// @resolution.member source=borrowed.total receiver=&'static constant Counter type=int32 kind=call target="Counter.total(parameters=(), arguments=(), return=int32)"
/// @resolution.place source=borrowed placement="constant" lifetime="static" access="mutable"
/// @resolution.access source=borrowed root=borrowed

const viewTotal = view.total;
/// @type.symbol symbol=viewTotal source=viewTotal type=int32
/// @resolution.pattern source=viewTotal kind=binding target=viewTotal
/// @resolution.name source=view target=view
/// @resolution.member source=view.total receiver=&'static readonly constant Counter type=int32 kind=call target="Counter.total(parameters=(), arguments=(), return=int32)"
/// @resolution.place source=view placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=view root=view

function mutate(owned: ^Counter, managed: Counter, borrowed: &Counter, view: &readonly Counter): void {
/// @generic.template symbol=mutate parameters=('a, 'b)
/// @type.symbol symbol=mutate type=<mutate.'a, mutate.'b>(Owned<Counter>, Counter, &mutate.'a Counter, &mutate.'b readonly Counter) => void
/// @type.symbol symbol=mutate.owned source="owned: ^Counter" type=Owned<Counter>
/// @resolution.name source=Counter target=Counter
/// @type.symbol symbol=mutate.managed source="managed: Counter" type=Counter
/// @resolution.name source=Counter target=Counter
/// @type.symbol symbol=mutate.borrowed source="borrowed: &Counter" type=&mutate.'a Counter
/// @resolution.name source=Counter target=Counter
/// @type.symbol symbol=mutate.view source="view: &readonly Counter" type=&mutate.'b readonly Counter
/// @resolution.name source=Counter target=Counter

    owned.bump();
    /// @resolution.name source=owned target=mutate.owned
    /// @resolution.member source=owned.bump receiver=Owned<Counter> type=<Counter.bump.'a>(this: &Counter.bump.'a exclusive Counter) => void kind=symbol target_receiver=Owned<Counter> target=Counter.bump
    /// @resolution.call source=owned.bump() parameters=() return=void kind=symbol target=Counter.bump receiver=Owned<Counter> adjustments=(borrow(&'frame exclusive Owned<Counter>))
    /// @resolution.place source=owned placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=owned root=mutate.owned

    managed.bump();
    /// @resolution.name source=managed target=mutate.managed
    /// @resolution.member source=managed.bump receiver=Counter type=<Counter.bump.'a>(this: &Counter.bump.'a exclusive Counter) => void kind=symbol target_receiver=Counter target=Counter.bump
    /// @resolution.call source=managed.bump() parameters=() return=void kind=symbol target=Counter.bump receiver=Counter adjustments=(borrow(&'frame exclusive Counter))
    /// @resolution.place source=managed placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=managed root=mutate.managed

    borrowed.bump();
    /// @resolution.name source=borrowed target=mutate.borrowed
    /// @resolution.member source=borrowed.bump receiver=&mutate.'a Counter type=<Counter.bump.'a>(this: &Counter.bump.'a exclusive Counter) => void kind=symbol target_receiver=&mutate.'a Counter target=Counter.bump
    /// @resolution.call source=borrowed.bump() parameters=() return=void kind=symbol target=Counter.bump receiver=&mutate.'a Counter
    /// @resolution.place source=borrowed placement=mutate.'a lifetime=mutate.'a access="mutable"
    /// @resolution.access source=borrowed root=mutate.borrowed

    view.bump();
    /// @resolution.name source=view target=mutate.view
    /// @resolution.member source=view.bump receiver=&mutate.'b readonly Counter type=<Counter.bump.'a>(this: &Counter.bump.'a exclusive Counter) => void kind=symbol target_receiver=&mutate.'b readonly Counter target=Counter.bump
    /// @resolution.call source=view.bump() parameters=() return=void kind=symbol target=Counter.bump receiver=&mutate.'b readonly Counter
    /// @resolution.place source=view placement=mutate.'b lifetime=mutate.'b access="readonly"
    /// @resolution.access source=view root=mutate.view

}
"#,
        r#"
/// @diagnostic.error id=receiver-not-assignable message="receiver type '&'a Counter' is not assignable to the method's 'this' type '&exclusive Counter'"
/// @diagnostic.label line=27 column=5 span="borrowed.bump()" line_source="borrowed.bump();"
/// @diagnostic.error id=receiver-not-assignable message="receiver type '&'b readonly Counter' is not assignable to the method's 'this' type '&exclusive Counter'"
/// @diagnostic.label line=28 column=5 span="view.bump()" line_source="view.bump();"
"#,
    );
}

#[test]
fn test_borrow_a_literal_argument_into_a_readonly_parameter() {
    let session = TestSession::single(
        r#"
declare function take(value: &readonly int32): void;
declare function takeText(value: &readonly string): void;
declare function make(): int32;

const stored: int32 = 1;

take(1);
take(stored);
take(make());
takeText("a");
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
declare function take<'a>(value: &'a readonly int32): void;
declare function takeText<'a>(value: &'a readonly string): void;
declare function make(): int32;

const stored: int32 = 1;

take(1 as &'frame readonly int32);
take(stored as &'static readonly int32);
take(make() as &'frame readonly int32);
takeText("a" as &'frame readonly string);

=== dir ===
declare function take(value: &readonly int32): void;
/// @generic.template symbol=take parameters=('a)
/// @type.symbol symbol=take source="declare function take(value: &readonly int32): void" type=<take.'a>(&take.'a readonly int32) => void
/// @type.symbol symbol=take.value source="value: &readonly int32" type=&take.'a readonly int32

declare function takeText(value: &readonly string): void;
/// @generic.template symbol=takeText parameters=('a)
/// @type.symbol symbol=takeText source="declare function takeText(value: &readonly string): void" type=<takeText.'a>(&takeText.'a readonly string) => void
/// @type.symbol symbol=takeText.value source="value: &readonly string" type=&takeText.'a readonly string

declare function make(): int32;
/// @type.symbol symbol=make source="declare function make(): int32" type=() => int32

const stored: int32 = 1;
/// @type.symbol symbol=stored source=stored type=int32
/// @resolution.pattern source=stored kind=binding target=stored
/// @coercion.node source=1 from=1 adjustments=[{ kind: materialize, target: int32 }] origin=implicit

take(1);
/// @resolution.name source=take target=take
/// @resolution.call source=take(1) parameters=(&'frame readonly int32) arguments=(provided(1) as &'frame readonly int32) return=void kind=symbol target=take
/// @coercion.node source=1 from=1 adjustments=[{ kind: materialize, target: int32 }, { kind: borrow, target: &'frame readonly int32 }] origin=implicit

take(stored);
/// @resolution.name source=take target=take
/// @resolution.call source=take(stored) parameters=(&'static readonly constant int32) arguments=(provided(stored) as &'static readonly constant int32) return=void kind=symbol target=take
/// @resolution.name source=stored target=stored
/// @resolution.place source=stored placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=stored root=stored
/// @coercion.node source=stored from=int32 adjustments=[{ kind: borrow, target: &'static readonly constant int32 }] origin=implicit

take(make());
/// @resolution.name source=take target=take
/// @resolution.call source=take(make()) parameters=(&'frame readonly int32) arguments=(provided(make()) as &'frame readonly int32) return=void kind=symbol target=take
/// @resolution.name source=make target=make
/// @resolution.call source=make() parameters=() return=int32 kind=symbol target=make
/// @coercion.node source=make() from=int32 adjustments=[{ kind: borrow, target: &'frame readonly int32 }] origin=implicit

takeText("a");
/// @resolution.name source=takeText target=takeText
/// @resolution.call source="takeText(\"a\")" parameters=(&'frame readonly string) arguments=(provided("a") as &'frame readonly string) return=void kind=symbol target=takeText
/// @coercion.node source="\"a\"" from="a" adjustments=[{ kind: materialize, target: string }, { kind: borrow, target: &'frame readonly string }] origin=implicit
"#,
        r#"

"#,
    );
}
