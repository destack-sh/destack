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
const ownedAt: int32 | undefined = owned.at<int32, "static">(0);
const ownedIncludes: boolean = owned.includes<int32, int32, "immutable", "static", "frame">(
    1 as &'frame immutable int32,
);

const managedLength: isize = managed.length;
const managedEmpty: boolean = managed.isEmpty;
const managedAt: int32 | undefined = managed.at<int32, "managed">(0);
const managedIncludes: boolean = managed.includes<int32, int32, "readonly", "managed", "frame">(
    1 as &'frame immutable int32,
);

const borrowedLength: isize = borrowed.length;
const borrowedEmpty: boolean = borrowed.isEmpty;
const borrowedAt: int32 | undefined = borrowed.at<int32, "static">(0);
const borrowedIncludes: boolean = borrowed.includes<int32, int32, "readonly", "static", "frame">(
    1 as &'frame immutable int32,
);

const viewLength: isize = view.length;
const viewEmpty: boolean = view.isEmpty;
const viewAt: int32 | undefined = view.at<int32, "static">(0);
const viewIncludes: boolean = view.includes<int32, int32, "readonly", "static", "frame">(
    1 as &'frame immutable int32,
);

=== dir ===
declare const owned: ^int32[];
/// @type.symbol symbol=owned source=owned type=^int32[]
/// @resolution.pattern source=owned kind=binding target=owned

declare const managed: int32[];
/// @type.symbol symbol=managed source=managed type=int32[]
/// @resolution.pattern source=managed kind=binding target=managed

declare const borrowed: &int32[];
/// @type.symbol symbol=borrowed source=borrowed type=&'static int32[]
/// @resolution.pattern source=borrowed kind=binding target=borrowed

declare const view: &readonly int32[];
/// @type.symbol symbol=view source=view type=&'static readonly int32[]
/// @resolution.pattern source=view kind=binding target=view

const ownedLength = owned.length;
/// @type.symbol symbol=ownedLength source=ownedLength type=isize
/// @resolution.pattern source=ownedLength kind=binding target=ownedLength
/// @resolution.name source=owned target=owned
/// @resolution.member source=owned.length receiver=^int32[] type=isize kind=call target="length(parameters=(), arguments=(), return=isize, regions=(\"static\" & \"local\"))"
/// @resolution.place source=owned placement="local" lifetime="static" access="immutable"
/// @resolution.access source=owned root=owned
/// @generic.instantiation id="length<int32, \"static\" & \"local\">" template=length arguments=(int32, "static" & "local")

const ownedEmpty = owned.isEmpty;
/// @type.symbol symbol=ownedEmpty source=ownedEmpty type=boolean
/// @resolution.pattern source=ownedEmpty kind=binding target=ownedEmpty
/// @resolution.name source=owned target=owned
/// @resolution.member source=owned.isEmpty receiver=^int32[] type=boolean kind=call target="isEmpty(parameters=(), arguments=(), return=boolean, regions=(\"static\" & \"local\"))"
/// @resolution.place source=owned placement="local" lifetime="static" access="immutable"
/// @resolution.access source=owned root=owned
/// @generic.instantiation id="isEmpty<int32, \"static\" & \"local\">" template=isEmpty arguments=(int32, "static" & "local")

const ownedAt = owned.at(0);
/// @type.symbol symbol=ownedAt source=ownedAt type=int32 | undefined
/// @resolution.pattern source=ownedAt kind=binding target=ownedAt
/// @resolution.name source=owned target=owned
/// @resolution.member source=owned.at receiver=^int32[] type=<at#1.'a>(this: &at#1.'a readonly int32[], isize) => int32 | undefined kind=symbol target_receiver=^int32[] target=at#1
/// @resolution.call source=owned.at(0) parameters=(isize) arguments=(provided(0) as isize) return=int32 | undefined regions=("static" & "local") kind=symbol target=at#1 receiver=^int32[] adjustments=(borrow(&'static readonly int32[])) instance="Array<int32>.<extension#4>.at#1<\"static\" & \"local\">"
/// @resolution.place source=owned placement="local" lifetime="static" access="immutable"
/// @resolution.access source=owned root=owned
/// @generic.instantiation id="at#1<int32, \"static\" & \"local\">" template=at#1 arguments=(int32, "static" & "local")
/// @generic.instantiation id=at#1<int32> template=at#1 arguments=(int32)

const ownedIncludes = owned.includes(1);
/// @type.symbol symbol=ownedIncludes source=ownedIncludes type=boolean
/// @resolution.pattern source=ownedIncludes kind=binding target=ownedIncludes
/// @resolution.name source=owned target=owned
/// @resolution.member source=owned.includes receiver=^int32[] type=<includes.Q, const includes.A: "readonly" | "immutable", includes.'a, includes.'b>(this: WithAccess<&includes.'a int32[], includes.A>, &includes.'b immutable includes.Q, isize | undefined?) => boolean kind=symbol target_receiver=^int32[] target=includes
/// @resolution.call source=owned.includes(1) parameters=(&'frame immutable int32, isize | undefined) arguments=(provided(1) as &'frame immutable int32, omitted as isize | undefined) return=boolean regions=("static" & "local", "frame" & "local") kind=symbol target=includes receiver=^int32[] adjustments=(borrow(&'static immutable int32[])) instance="Array<int32>.<extension#6>.includes<int32, \"immutable\", \"static\" & \"local\", \"frame\" & \"local\">"
/// @resolution.place source=owned placement="local" lifetime="static" access="immutable"
/// @resolution.access source=owned root=owned
/// @generic.instantiation id="includes<int32, int32, \"immutable\", \"static\" & \"local\", \"frame\" & \"local\">" template=includes arguments=(int32, int32, "immutable", "static" & "local", "frame" & "local")
/// @generic.instantiation id=includes<int32> template=includes arguments=(int32)

const managedLength = managed.length;
/// @type.symbol symbol=managedLength source=managedLength type=isize
/// @resolution.pattern source=managedLength kind=binding target=managedLength
/// @resolution.name source=managed target=managed
/// @resolution.member source=managed.length receiver=int32[] type=isize kind=call target="length(parameters=(), arguments=(), return=isize, regions=(\"managed\" & \"local\"))"
/// @resolution.place source=managed placement="local" lifetime="static" access="immutable"
/// @resolution.access source=managed root=managed
/// @generic.instantiation id="length<int32, \"managed\" & \"local\">" template=length arguments=(int32, "managed" & "local")

const managedEmpty = managed.isEmpty;
/// @type.symbol symbol=managedEmpty source=managedEmpty type=boolean
/// @resolution.pattern source=managedEmpty kind=binding target=managedEmpty
/// @resolution.name source=managed target=managed
/// @resolution.member source=managed.isEmpty receiver=int32[] type=boolean kind=call target="isEmpty(parameters=(), arguments=(), return=boolean, regions=(\"managed\" & \"local\"))"
/// @resolution.place source=managed placement="local" lifetime="static" access="immutable"
/// @resolution.access source=managed root=managed
/// @generic.instantiation id="isEmpty<int32, \"managed\" & \"local\">" template=isEmpty arguments=(int32, "managed" & "local")

const managedAt = managed.at(0);
/// @type.symbol symbol=managedAt source=managedAt type=int32 | undefined
/// @resolution.pattern source=managedAt kind=binding target=managedAt
/// @resolution.name source=managed target=managed
/// @resolution.member source=managed.at receiver=int32[] type=<at#1.'a>(this: &at#1.'a readonly int32[], isize) => int32 | undefined kind=symbol target_receiver=int32[] target=at#1
/// @resolution.call source=managed.at(0) parameters=(isize) arguments=(provided(0) as isize) return=int32 | undefined regions=("managed" & "local") kind=symbol target=at#1 receiver=int32[] adjustments=(borrow(&'managed readonly int32[])) instance="Array<int32>.<extension#4>.at#1<\"managed\" & \"local\">"
/// @resolution.place source=managed placement="local" lifetime="static" access="immutable"
/// @resolution.access source=managed root=managed
/// @generic.instantiation id="at#1<int32, \"managed\" & \"local\">" template=at#1 arguments=(int32, "managed" & "local")

const managedIncludes = managed.includes(1);
/// @type.symbol symbol=managedIncludes source=managedIncludes type=boolean
/// @resolution.pattern source=managedIncludes kind=binding target=managedIncludes
/// @resolution.name source=managed target=managed
/// @resolution.member source=managed.includes receiver=int32[] type=<includes.Q, const includes.A: "readonly" | "immutable", includes.'a, includes.'b>(this: WithAccess<&includes.'a int32[], includes.A>, &includes.'b immutable includes.Q, isize | undefined?) => boolean kind=symbol target_receiver=int32[] target=includes
/// @resolution.call source=managed.includes(1) parameters=(&'frame immutable int32, isize | undefined) arguments=(provided(1) as &'frame immutable int32, omitted as isize | undefined) return=boolean regions=("managed" & "local", "frame" & "local") kind=symbol target=includes receiver=int32[] adjustments=(borrow(&'managed readonly int32[])) instance="Array<int32>.<extension#6>.includes<int32, \"readonly\", \"managed\" & \"local\", \"frame\" & \"local\">"
/// @resolution.place source=managed placement="local" lifetime="static" access="immutable"
/// @resolution.access source=managed root=managed
/// @generic.instantiation id="includes<int32, int32, \"readonly\", \"managed\" & \"local\", \"frame\" & \"local\">" template=includes arguments=(int32, int32, "readonly", "managed" & "local", "frame" & "local")

const borrowedLength = borrowed.length;
/// @type.symbol symbol=borrowedLength source=borrowedLength type=isize
/// @resolution.pattern source=borrowedLength kind=binding target=borrowedLength
/// @resolution.name source=borrowed target=borrowed
/// @resolution.member source=borrowed.length receiver=&'static int32[] type=isize kind=call target="length(parameters=(), arguments=(), return=isize, regions=(\"static\" & \"local\"))"
/// @resolution.place source=borrowed placement="local" lifetime="static" access="immutable"
/// @resolution.access source=borrowed root=borrowed

const borrowedEmpty = borrowed.isEmpty;
/// @type.symbol symbol=borrowedEmpty source=borrowedEmpty type=boolean
/// @resolution.pattern source=borrowedEmpty kind=binding target=borrowedEmpty
/// @resolution.name source=borrowed target=borrowed
/// @resolution.member source=borrowed.isEmpty receiver=&'static int32[] type=boolean kind=call target="isEmpty(parameters=(), arguments=(), return=boolean, regions=(\"static\" & \"local\"))"
/// @resolution.place source=borrowed placement="local" lifetime="static" access="immutable"
/// @resolution.access source=borrowed root=borrowed

const borrowedAt = borrowed.at(0);
/// @type.symbol symbol=borrowedAt source=borrowedAt type=int32 | undefined
/// @resolution.pattern source=borrowedAt kind=binding target=borrowedAt
/// @resolution.name source=borrowed target=borrowed
/// @resolution.member source=borrowed.at receiver=&'static int32[] type=<at#1.'a>(this: &at#1.'a readonly int32[], isize) => int32 | undefined kind=symbol target_receiver=&'static int32[] target=at#1
/// @resolution.call source=borrowed.at(0) parameters=(isize) arguments=(provided(0) as isize) return=int32 | undefined regions=("static" & "local") kind=symbol target=at#1 receiver=&'static int32[] instance="Array<int32>.<extension#4>.at#1<\"static\" & \"local\">"
/// @resolution.place source=borrowed placement="local" lifetime="static" access="immutable"
/// @resolution.access source=borrowed root=borrowed

const borrowedIncludes = borrowed.includes(1);
/// @type.symbol symbol=borrowedIncludes source=borrowedIncludes type=boolean
/// @resolution.pattern source=borrowedIncludes kind=binding target=borrowedIncludes
/// @resolution.name source=borrowed target=borrowed
/// @resolution.member source=borrowed.includes receiver=&'static int32[] type=<includes.Q, const includes.A: "readonly" | "immutable", includes.'a, includes.'b>(this: WithAccess<&includes.'a int32[], includes.A>, &includes.'b immutable includes.Q, isize | undefined?) => boolean kind=symbol target_receiver=&'static int32[] target=includes
/// @resolution.call source=borrowed.includes(1) parameters=(&'frame immutable int32, isize | undefined) arguments=(provided(1) as &'frame immutable int32, omitted as isize | undefined) return=boolean regions=("static" & "local", "frame" & "local") kind=symbol target=includes receiver=&'static int32[] instance="Array<int32>.<extension#6>.includes<int32, \"readonly\", \"static\" & \"local\", \"frame\" & \"local\">"
/// @resolution.place source=borrowed placement="local" lifetime="static" access="immutable"
/// @resolution.access source=borrowed root=borrowed
/// @generic.instantiation id="includes<int32, int32, \"readonly\", \"static\" & \"local\", \"frame\" & \"local\">" template=includes arguments=(int32, int32, "readonly", "static" & "local", "frame" & "local")

const viewLength = view.length;
/// @type.symbol symbol=viewLength source=viewLength type=isize
/// @resolution.pattern source=viewLength kind=binding target=viewLength
/// @resolution.name source=view target=view
/// @resolution.member source=view.length receiver=&'static readonly int32[] type=isize kind=call target="length(parameters=(), arguments=(), return=isize, regions=(\"static\" & \"local\"))"
/// @resolution.place source=view placement="local" lifetime="static" access="immutable"
/// @resolution.access source=view root=view

const viewEmpty = view.isEmpty;
/// @type.symbol symbol=viewEmpty source=viewEmpty type=boolean
/// @resolution.pattern source=viewEmpty kind=binding target=viewEmpty
/// @resolution.name source=view target=view
/// @resolution.member source=view.isEmpty receiver=&'static readonly int32[] type=boolean kind=call target="isEmpty(parameters=(), arguments=(), return=boolean, regions=(\"static\" & \"local\"))"
/// @resolution.place source=view placement="local" lifetime="static" access="immutable"
/// @resolution.access source=view root=view

const viewAt = view.at(0);
/// @type.symbol symbol=viewAt source=viewAt type=int32 | undefined
/// @resolution.pattern source=viewAt kind=binding target=viewAt
/// @resolution.name source=view target=view
/// @resolution.member source=view.at receiver=&'static readonly int32[] type=<at#1.'a>(this: &at#1.'a readonly int32[], isize) => int32 | undefined kind=symbol target_receiver=&'static readonly int32[] target=at#1
/// @resolution.call source=view.at(0) parameters=(isize) arguments=(provided(0) as isize) return=int32 | undefined regions=("static" & "local") kind=symbol target=at#1 receiver=&'static readonly int32[] instance="Array<int32>.<extension#4>.at#1<\"static\" & \"local\">"
/// @resolution.place source=view placement="local" lifetime="static" access="immutable"
/// @resolution.access source=view root=view

const viewIncludes = view.includes(1);
/// @type.symbol symbol=viewIncludes source=viewIncludes type=boolean
/// @resolution.pattern source=viewIncludes kind=binding target=viewIncludes
/// @resolution.name source=view target=view
/// @resolution.member source=view.includes receiver=&'static readonly int32[] type=<includes.Q, const includes.A: "readonly" | "immutable", includes.'a, includes.'b>(this: WithAccess<&includes.'a int32[], includes.A>, &includes.'b immutable includes.Q, isize | undefined?) => boolean kind=symbol target_receiver=&'static readonly int32[] target=includes
/// @resolution.call source=view.includes(1) parameters=(&'frame immutable int32, isize | undefined) arguments=(provided(1) as &'frame immutable int32, omitted as isize | undefined) return=boolean regions=("static" & "local", "frame" & "local") kind=symbol target=includes receiver=&'static readonly int32[] instance="Array<int32>.<extension#6>.includes<int32, \"readonly\", \"static\" & \"local\", \"frame\" & \"local\">"
/// @resolution.place source=view placement="local" lifetime="static" access="immutable"
/// @resolution.access source=view root=view
"#,
        r#"

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
const ownedIncludes: boolean = owned.includes<"static", "managed">(
    "a" as &'managed readonly string,
);
const ownedTrimmed: ^string = owned.trim<"static">();

const managedLength: isize = managed.length;
const managedEmpty: boolean = managed.isEmpty;
const managedIncludes: boolean = managed.includes<"managed", "managed">(
    "a" as &'managed readonly string,
);
const managedTrimmed: ^string = managed.trim<"managed">();

const viewLength: isize = view.length;
const viewEmpty: boolean = view.isEmpty;
const viewIncludes: boolean = view.includes<"static", "managed">("a" as &'managed readonly string);
const viewTrimmed: ^string = view.trim<"static">();

=== dir ===
declare const owned: ^string;
/// @type.symbol symbol=owned source=owned type=^string
/// @resolution.pattern source=owned kind=binding target=owned

declare const managed: string;
/// @type.symbol symbol=managed source=managed type=string
/// @resolution.pattern source=managed kind=binding target=managed

declare const view: &readonly string;
/// @type.symbol symbol=view source=view type=&'static readonly string
/// @resolution.pattern source=view kind=binding target=view

const ownedLength = owned.length;
/// @type.symbol symbol=ownedLength source=ownedLength type=isize
/// @resolution.pattern source=ownedLength kind=binding target=ownedLength
/// @resolution.name source=owned target=owned
/// @resolution.member source=owned.length receiver=^string type=isize kind=call target="length(parameters=(), arguments=(), return=isize, regions=(\"static\" & \"local\"))"
/// @resolution.place source=owned placement="local" lifetime="static" access="immutable"
/// @resolution.access source=owned root=owned
/// @generic.instantiation id="length<\"static\" & \"local\">" template=length arguments=("static" & "local")

const ownedEmpty = owned.isEmpty;
/// @type.symbol symbol=ownedEmpty source=ownedEmpty type=boolean
/// @resolution.pattern source=ownedEmpty kind=binding target=ownedEmpty
/// @resolution.name source=owned target=owned
/// @resolution.member source=owned.isEmpty receiver=^string type=boolean kind=call target="isEmpty(parameters=(), arguments=(), return=boolean, regions=(\"static\" & \"local\"))"
/// @resolution.place source=owned placement="local" lifetime="static" access="immutable"
/// @resolution.access source=owned root=owned
/// @generic.instantiation id="isEmpty<\"static\" & \"local\">" template=isEmpty arguments=("static" & "local")

const ownedIncludes = owned.includes("a");
/// @type.symbol symbol=ownedIncludes source=ownedIncludes type=boolean
/// @resolution.pattern source=ownedIncludes kind=binding target=ownedIncludes
/// @resolution.name source=owned target=owned
/// @resolution.member source=owned.includes receiver=^string type=<includes.'a, includes.'b>(this: &includes.'a readonly string, &includes.'b readonly string, isize | undefined?) => boolean kind=symbol target_receiver=^string target=includes
/// @resolution.call source="owned.includes(\"a\")" parameters=(&'managed readonly string, isize | undefined) arguments=(provided("a") as &'managed readonly string, omitted as isize | undefined) return=boolean regions=("static" & "local", "managed" & "local") kind=symbol target=includes receiver=^string adjustments=(borrow(&'static readonly string)) instance="string.<extension#2>.includes<\"static\" & \"local\", \"managed\" & \"local\">"
/// @resolution.place source=owned placement="local" lifetime="static" access="immutable"
/// @resolution.access source=owned root=owned
/// @generic.instantiation id="includes<\"static\" & \"local\", \"managed\" & \"local\">" template=includes arguments=("static" & "local", "managed" & "local")

const ownedTrimmed = owned.trim();
/// @type.symbol symbol=ownedTrimmed source=ownedTrimmed type=^string
/// @resolution.pattern source=ownedTrimmed kind=binding target=ownedTrimmed
/// @resolution.name source=owned target=owned
/// @resolution.member source=owned.trim receiver=^string type=<trim.'a>(this: &trim.'a readonly string) => ^string kind=symbol target_receiver=^string target=trim
/// @resolution.call source=owned.trim() parameters=() return=^string regions=("static" & "local") kind=symbol target=trim receiver=^string adjustments=(borrow(&'static readonly string)) instance="string.<extension#2>.trim<\"static\" & \"local\">"
/// @resolution.place source=owned placement="local" lifetime="static" access="immutable"
/// @resolution.access source=owned root=owned
/// @generic.instantiation id="trim<\"static\" & \"local\">" template=trim arguments=("static" & "local")

const managedLength = managed.length;
/// @type.symbol symbol=managedLength source=managedLength type=isize
/// @resolution.pattern source=managedLength kind=binding target=managedLength
/// @resolution.name source=managed target=managed
/// @resolution.member source=managed.length receiver=string type=isize kind=call target="length(parameters=(), arguments=(), return=isize, regions=(\"managed\" & \"local\"))"
/// @resolution.place source=managed placement="local" lifetime="static" access="immutable"
/// @resolution.access source=managed root=managed
/// @generic.instantiation id="length<\"managed\" & \"local\">" template=length arguments=("managed" & "local")

const managedEmpty = managed.isEmpty;
/// @type.symbol symbol=managedEmpty source=managedEmpty type=boolean
/// @resolution.pattern source=managedEmpty kind=binding target=managedEmpty
/// @resolution.name source=managed target=managed
/// @resolution.member source=managed.isEmpty receiver=string type=boolean kind=call target="isEmpty(parameters=(), arguments=(), return=boolean, regions=(\"managed\" & \"local\"))"
/// @resolution.place source=managed placement="local" lifetime="static" access="immutable"
/// @resolution.access source=managed root=managed
/// @generic.instantiation id="isEmpty<\"managed\" & \"local\">" template=isEmpty arguments=("managed" & "local")

const managedIncludes = managed.includes("a");
/// @type.symbol symbol=managedIncludes source=managedIncludes type=boolean
/// @resolution.pattern source=managedIncludes kind=binding target=managedIncludes
/// @resolution.name source=managed target=managed
/// @resolution.member source=managed.includes receiver=string type=<includes.'a, includes.'b>(this: &includes.'a readonly string, &includes.'b readonly string, isize | undefined?) => boolean kind=symbol target_receiver=string target=includes
/// @resolution.call source="managed.includes(\"a\")" parameters=(&'managed readonly string, isize | undefined) arguments=(provided("a") as &'managed readonly string, omitted as isize | undefined) return=boolean regions=("managed" & "local", "managed" & "local") kind=symbol target=includes receiver=string adjustments=(borrow(&'managed readonly string)) instance="string.<extension#2>.includes<\"managed\" & \"local\", \"managed\" & \"local\">"
/// @resolution.place source=managed placement="local" lifetime="static" access="immutable"
/// @resolution.access source=managed root=managed
/// @generic.instantiation id="includes<\"managed\" & \"local\", \"managed\" & \"local\">" template=includes arguments=("managed" & "local", "managed" & "local")

const managedTrimmed = managed.trim();
/// @type.symbol symbol=managedTrimmed source=managedTrimmed type=^string
/// @resolution.pattern source=managedTrimmed kind=binding target=managedTrimmed
/// @resolution.name source=managed target=managed
/// @resolution.member source=managed.trim receiver=string type=<trim.'a>(this: &trim.'a readonly string) => ^string kind=symbol target_receiver=string target=trim
/// @resolution.call source=managed.trim() parameters=() return=^string regions=("managed" & "local") kind=symbol target=trim receiver=string adjustments=(borrow(&'managed readonly string)) instance="string.<extension#2>.trim<\"managed\" & \"local\">"
/// @resolution.place source=managed placement="local" lifetime="static" access="immutable"
/// @resolution.access source=managed root=managed
/// @generic.instantiation id="trim<\"managed\" & \"local\">" template=trim arguments=("managed" & "local")

const viewLength = view.length;
/// @type.symbol symbol=viewLength source=viewLength type=isize
/// @resolution.pattern source=viewLength kind=binding target=viewLength
/// @resolution.name source=view target=view
/// @resolution.member source=view.length receiver=&'static readonly string type=isize kind=call target="length(parameters=(), arguments=(), return=isize, regions=(\"static\" & \"local\"))"
/// @resolution.place source=view placement="local" lifetime="static" access="immutable"
/// @resolution.access source=view root=view

const viewEmpty = view.isEmpty;
/// @type.symbol symbol=viewEmpty source=viewEmpty type=boolean
/// @resolution.pattern source=viewEmpty kind=binding target=viewEmpty
/// @resolution.name source=view target=view
/// @resolution.member source=view.isEmpty receiver=&'static readonly string type=boolean kind=call target="isEmpty(parameters=(), arguments=(), return=boolean, regions=(\"static\" & \"local\"))"
/// @resolution.place source=view placement="local" lifetime="static" access="immutable"
/// @resolution.access source=view root=view

const viewIncludes = view.includes("a");
/// @type.symbol symbol=viewIncludes source=viewIncludes type=boolean
/// @resolution.pattern source=viewIncludes kind=binding target=viewIncludes
/// @resolution.name source=view target=view
/// @resolution.member source=view.includes receiver=&'static readonly string type=<includes.'a, includes.'b>(this: &includes.'a readonly string, &includes.'b readonly string, isize | undefined?) => boolean kind=symbol target_receiver=&'static readonly string target=includes
/// @resolution.call source="view.includes(\"a\")" parameters=(&'managed readonly string, isize | undefined) arguments=(provided("a") as &'managed readonly string, omitted as isize | undefined) return=boolean regions=("static" & "local", "managed" & "local") kind=symbol target=includes receiver=&'static readonly string instance="string.<extension#2>.includes<\"static\" & \"local\", \"managed\" & \"local\">"
/// @resolution.place source=view placement="local" lifetime="static" access="immutable"
/// @resolution.access source=view root=view

const viewTrimmed = view.trim();
/// @type.symbol symbol=viewTrimmed source=viewTrimmed type=^string
/// @resolution.pattern source=viewTrimmed kind=binding target=viewTrimmed
/// @resolution.name source=view target=view
/// @resolution.member source=view.trim receiver=&'static readonly string type=<trim.'a>(this: &trim.'a readonly string) => ^string kind=symbol target_receiver=&'static readonly string target=trim
/// @resolution.call source=view.trim() parameters=() return=^string regions=("static" & "local") kind=symbol target=trim receiver=&'static readonly string instance="string.<extension#2>.trim<\"static\" & \"local\">"
/// @resolution.place source=view placement="local" lifetime="static" access="immutable"
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

declare const ownedMap: ^Map<string, int32, Equality<string>>;
declare const managedMap: Map<string, int32, Equality<string>>;
declare const viewMap: &'static readonly Map<string, int32, Equality<string>>;
declare const ownedSet: ^Set<int32, Equality<int32>>;
declare const managedSet: Set<int32, Equality<int32>>;
declare const viewSet: &'static readonly Set<int32, Equality<int32>>;

const ownedMapSize: usize = ownedMap.size;
const ownedMapHas: boolean = ownedMap.has<
    string,
    int32,
    Equality<string>,
    string,
    "immutable",
    "static"
>("a");
const managedMapSize: usize = managedMap.size;
const managedMapHas: boolean = managedMap.has<
    string,
    int32,
    Equality<string>,
    string,
    "readonly",
    "managed"
>("a");
const viewMapSize: usize = viewMap.size;
const viewMapHas: boolean = viewMap.has<
    string,
    int32,
    Equality<string>,
    string,
    "readonly",
    "static"
>("a");

const ownedSetSize: usize = ownedSet.size;
const ownedSetHas: boolean = ownedSet.has<int32, Equality<int32>, int32, "immutable", "static">(1);
const managedSetSize: usize = managedSet.size;
const managedSetHas: boolean = managedSet.has<int32, Equality<int32>, int32, "readonly", "managed">(
    1,
);
const viewSetSize: usize = viewSet.size;
const viewSetHas: boolean = viewSet.has<int32, Equality<int32>, int32, "readonly", "static">(1);

=== dir ===
import { Map, Set } from "destack:collections";

declare const ownedMap: ^Map<string, int32>;
/// @type.symbol symbol=ownedMap source=ownedMap type=^Map<string, int32, Equality<string>>
/// @resolution.pattern source=ownedMap kind=binding target=ownedMap
/// @resolution.name source=Map target=Map

declare const managedMap: Map<string, int32>;
/// @type.symbol symbol=managedMap source=managedMap type=Map<string, int32, Equality<string>>
/// @resolution.pattern source=managedMap kind=binding target=managedMap
/// @resolution.name source=Map target=Map

declare const viewMap: &readonly Map<string, int32>;
/// @type.symbol symbol=viewMap source=viewMap type=&'static readonly Map<string, int32, Equality<string>>
/// @resolution.pattern source=viewMap kind=binding target=viewMap
/// @resolution.name source=Map target=Map

declare const ownedSet: ^Set<int32>;
/// @type.symbol symbol=ownedSet source=ownedSet type=^Set<int32, Equality<int32>>
/// @resolution.pattern source=ownedSet kind=binding target=ownedSet
/// @resolution.name source=Set target=Set

declare const managedSet: Set<int32>;
/// @type.symbol symbol=managedSet source=managedSet type=Set<int32, Equality<int32>>
/// @resolution.pattern source=managedSet kind=binding target=managedSet
/// @resolution.name source=Set target=Set

declare const viewSet: &readonly Set<int32>;
/// @type.symbol symbol=viewSet source=viewSet type=&'static readonly Set<int32, Equality<int32>>
/// @resolution.pattern source=viewSet kind=binding target=viewSet
/// @resolution.name source=Set target=Set

const ownedMapSize = ownedMap.size;
/// @type.symbol symbol=ownedMapSize source=ownedMapSize type=usize
/// @resolution.pattern source=ownedMapSize kind=binding target=ownedMapSize
/// @resolution.name source=ownedMap target=ownedMap
/// @resolution.member source=ownedMap.size receiver=^Map<string, int32, Equality<string>> type=usize kind=call target="size(parameters=(), arguments=(), return=usize, regions=(\"static\" & \"local\"))"
/// @resolution.place source=ownedMap placement="local" lifetime="static" access="immutable"
/// @resolution.access source=ownedMap root=ownedMap
/// @generic.instantiation id="size<string, int32, Equality<string>, \"static\" & \"local\">" template=size arguments=(string, int32, Equality<string>, "static" & "local")

const ownedMapHas = ownedMap.has("a");
/// @type.symbol symbol=ownedMapHas source=ownedMapHas type=boolean
/// @resolution.pattern source=ownedMapHas kind=binding target=ownedMapHas
/// @resolution.name source=ownedMap target=ownedMap
/// @resolution.member source=ownedMap.has receiver=^Map<string, int32, Equality<string>> type=<has.Q#1, const has.A#1: "readonly" | "immutable", has#1.'a>(this: WithAccess<&has#1.'a readonly Map<string, int32, Equality<string>>, has.A#1>, has.Q#1) => boolean & <has.Q#2, const has.A#2: "readonly" | "immutable", has#2.'a, has#2.'b>(this: WithAccess<&has#2.'a readonly Map<string, int32, Equality<string>>, has.A#2>, &has#2.'b immutable has.Q#2) => boolean kind=overload-set targets=[has#1, has#2]
/// @resolution.call source="ownedMap.has(\"a\")" parameters=(string) arguments=(provided("a") as string) return=boolean regions=("static" & "local") kind=symbol target=has#1 receiver=^Map<string, int32, Equality<string>> adjustments=(borrow(&'static immutable Map<string, int32, Equality<string>>)) instance="Map<string, int32, Equality<string>>.<extension#9>.has#1<string, \"immutable\", \"static\" & \"local\">"
/// @resolution.place source=ownedMap placement="local" lifetime="static" access="immutable"
/// @resolution.access source=ownedMap root=ownedMap
/// @generic.instantiation id="has#1<string, int32, Equality<string>, string, \"immutable\", \"static\" & \"local\">" template=has#1 arguments=(string, int32, Equality<string>, string, "immutable", "static" & "local")
/// @generic.instantiation id="has#1<string, int32, Equality<string>>" template=has#1 arguments=(string, int32, Equality<string>)
/// @generic.instantiation id="has#2<string, int32, Equality<string>>" template=has#2 arguments=(string, int32, Equality<string>)

const managedMapSize = managedMap.size;
/// @type.symbol symbol=managedMapSize source=managedMapSize type=usize
/// @resolution.pattern source=managedMapSize kind=binding target=managedMapSize
/// @resolution.name source=managedMap target=managedMap
/// @resolution.member source=managedMap.size receiver=Map<string, int32, Equality<string>> type=usize kind=call target="size(parameters=(), arguments=(), return=usize, regions=(\"managed\" & \"local\"))"
/// @resolution.place source=managedMap placement="local" lifetime="static" access="immutable"
/// @resolution.access source=managedMap root=managedMap
/// @generic.instantiation id="size<string, int32, Equality<string>, \"managed\" & \"local\">" template=size arguments=(string, int32, Equality<string>, "managed" & "local")

const managedMapHas = managedMap.has("a");
/// @type.symbol symbol=managedMapHas source=managedMapHas type=boolean
/// @resolution.pattern source=managedMapHas kind=binding target=managedMapHas
/// @resolution.name source=managedMap target=managedMap
/// @resolution.member source=managedMap.has receiver=Map<string, int32, Equality<string>> type=<has.Q#1, const has.A#1: "readonly" | "immutable", has#1.'a>(this: WithAccess<&has#1.'a readonly Map<string, int32, Equality<string>>, has.A#1>, has.Q#1) => boolean & <has.Q#2, const has.A#2: "readonly" | "immutable", has#2.'a, has#2.'b>(this: WithAccess<&has#2.'a readonly Map<string, int32, Equality<string>>, has.A#2>, &has#2.'b immutable has.Q#2) => boolean kind=overload-set targets=[has#1, has#2]
/// @resolution.call source="managedMap.has(\"a\")" parameters=(string) arguments=(provided("a") as string) return=boolean regions=("managed" & "local") kind=symbol target=has#1 receiver=Map<string, int32, Equality<string>> adjustments=(borrow(&'managed readonly Map<string, int32, Equality<string>>)) instance="Map<string, int32, Equality<string>>.<extension#9>.has#1<string, \"readonly\", \"managed\" & \"local\">"
/// @resolution.place source=managedMap placement="local" lifetime="static" access="immutable"
/// @resolution.access source=managedMap root=managedMap
/// @generic.instantiation id="has#1<string, int32, Equality<string>, string, \"readonly\", \"managed\" & \"local\">" template=has#1 arguments=(string, int32, Equality<string>, string, "readonly", "managed" & "local")

const viewMapSize = viewMap.size;
/// @type.symbol symbol=viewMapSize source=viewMapSize type=usize
/// @resolution.pattern source=viewMapSize kind=binding target=viewMapSize
/// @resolution.name source=viewMap target=viewMap
/// @resolution.member source=viewMap.size receiver=&'static readonly Map<string, int32, Equality<string>> type=usize kind=call target="size(parameters=(), arguments=(), return=usize, regions=(\"static\" & \"local\"))"
/// @resolution.place source=viewMap placement="local" lifetime="static" access="immutable"
/// @resolution.access source=viewMap root=viewMap

const viewMapHas = viewMap.has("a");
/// @type.symbol symbol=viewMapHas source=viewMapHas type=boolean
/// @resolution.pattern source=viewMapHas kind=binding target=viewMapHas
/// @resolution.name source=viewMap target=viewMap
/// @resolution.member source=viewMap.has receiver=&'static readonly Map<string, int32, Equality<string>> type=<has.Q#1, const has.A#1: "readonly" | "immutable", has#1.'a>(this: WithAccess<&has#1.'a readonly Map<string, int32, Equality<string>>, has.A#1>, has.Q#1) => boolean & <has.Q#2, const has.A#2: "readonly" | "immutable", has#2.'a, has#2.'b>(this: WithAccess<&has#2.'a readonly Map<string, int32, Equality<string>>, has.A#2>, &has#2.'b immutable has.Q#2) => boolean kind=overload-set targets=[has#1, has#2]
/// @resolution.call source="viewMap.has(\"a\")" parameters=(string) arguments=(provided("a") as string) return=boolean regions=("static" & "local") kind=symbol target=has#1 receiver=&'static readonly Map<string, int32, Equality<string>> instance="Map<string, int32, Equality<string>>.<extension#9>.has#1<string, \"readonly\", \"static\" & \"local\">"
/// @resolution.place source=viewMap placement="local" lifetime="static" access="immutable"
/// @resolution.access source=viewMap root=viewMap
/// @generic.instantiation id="has#1<string, int32, Equality<string>, string, \"readonly\", \"static\" & \"local\">" template=has#1 arguments=(string, int32, Equality<string>, string, "readonly", "static" & "local")

const ownedSetSize = ownedSet.size;
/// @type.symbol symbol=ownedSetSize source=ownedSetSize type=usize
/// @resolution.pattern source=ownedSetSize kind=binding target=ownedSetSize
/// @resolution.name source=ownedSet target=ownedSet
/// @resolution.member source=ownedSet.size receiver=^Set<int32, Equality<int32>> type=usize kind=call target="size(parameters=(), arguments=(), return=usize, regions=(\"static\" & \"local\"))"
/// @resolution.place source=ownedSet placement="local" lifetime="static" access="immutable"
/// @resolution.access source=ownedSet root=ownedSet
/// @generic.instantiation id="size<int32, Equality<int32>, \"static\" & \"local\">" template=size arguments=(int32, Equality<int32>, "static" & "local")

const ownedSetHas = ownedSet.has(1);
/// @type.symbol symbol=ownedSetHas source=ownedSetHas type=boolean
/// @resolution.pattern source=ownedSetHas kind=binding target=ownedSetHas
/// @resolution.name source=ownedSet target=ownedSet
/// @resolution.member source=ownedSet.has receiver=^Set<int32, Equality<int32>> type=<has.Q#1, const has.A#1: "readonly" | "immutable", has#1.'a>(this: WithAccess<&has#1.'a readonly Set<int32, Equality<int32>>, has.A#1>, has.Q#1) => boolean & <has.Q#2, const has.A#2: "readonly" | "immutable", has#2.'a, has#2.'b>(this: WithAccess<&has#2.'a readonly Set<int32, Equality<int32>>, has.A#2>, &has#2.'b immutable has.Q#2) => boolean kind=overload-set targets=[has#1, has#2]
/// @resolution.call source=ownedSet.has(1) parameters=(int32) arguments=(provided(1) as int32) return=boolean regions=("static" & "local") kind=symbol target=has#1 receiver=^Set<int32, Equality<int32>> adjustments=(borrow(&'static immutable Set<int32, Equality<int32>>)) instance="Set<int32, Equality<int32>>.<extension#6>.has#1<int32, \"immutable\", \"static\" & \"local\">"
/// @resolution.place source=ownedSet placement="local" lifetime="static" access="immutable"
/// @resolution.access source=ownedSet root=ownedSet
/// @generic.instantiation id="has#1<int32, Equality<int32>, int32, \"immutable\", \"static\" & \"local\">" template=has#1 arguments=(int32, Equality<int32>, int32, "immutable", "static" & "local")
/// @generic.instantiation id="has#1<int32, Equality<int32>>" template=has#1 arguments=(int32, Equality<int32>)
/// @generic.instantiation id="has#2<int32, Equality<int32>>" template=has#2 arguments=(int32, Equality<int32>)

const managedSetSize = managedSet.size;
/// @type.symbol symbol=managedSetSize source=managedSetSize type=usize
/// @resolution.pattern source=managedSetSize kind=binding target=managedSetSize
/// @resolution.name source=managedSet target=managedSet
/// @resolution.member source=managedSet.size receiver=Set<int32, Equality<int32>> type=usize kind=call target="size(parameters=(), arguments=(), return=usize, regions=(\"managed\" & \"local\"))"
/// @resolution.place source=managedSet placement="local" lifetime="static" access="immutable"
/// @resolution.access source=managedSet root=managedSet
/// @generic.instantiation id="size<int32, Equality<int32>, \"managed\" & \"local\">" template=size arguments=(int32, Equality<int32>, "managed" & "local")

const managedSetHas = managedSet.has(1);
/// @type.symbol symbol=managedSetHas source=managedSetHas type=boolean
/// @resolution.pattern source=managedSetHas kind=binding target=managedSetHas
/// @resolution.name source=managedSet target=managedSet
/// @resolution.member source=managedSet.has receiver=Set<int32, Equality<int32>> type=<has.Q#1, const has.A#1: "readonly" | "immutable", has#1.'a>(this: WithAccess<&has#1.'a readonly Set<int32, Equality<int32>>, has.A#1>, has.Q#1) => boolean & <has.Q#2, const has.A#2: "readonly" | "immutable", has#2.'a, has#2.'b>(this: WithAccess<&has#2.'a readonly Set<int32, Equality<int32>>, has.A#2>, &has#2.'b immutable has.Q#2) => boolean kind=overload-set targets=[has#1, has#2]
/// @resolution.call source=managedSet.has(1) parameters=(int32) arguments=(provided(1) as int32) return=boolean regions=("managed" & "local") kind=symbol target=has#1 receiver=Set<int32, Equality<int32>> adjustments=(borrow(&'managed readonly Set<int32, Equality<int32>>)) instance="Set<int32, Equality<int32>>.<extension#6>.has#1<int32, \"readonly\", \"managed\" & \"local\">"
/// @resolution.place source=managedSet placement="local" lifetime="static" access="immutable"
/// @resolution.access source=managedSet root=managedSet
/// @generic.instantiation id="has#1<int32, Equality<int32>, int32, \"readonly\", \"managed\" & \"local\">" template=has#1 arguments=(int32, Equality<int32>, int32, "readonly", "managed" & "local")

const viewSetSize = viewSet.size;
/// @type.symbol symbol=viewSetSize source=viewSetSize type=usize
/// @resolution.pattern source=viewSetSize kind=binding target=viewSetSize
/// @resolution.name source=viewSet target=viewSet
/// @resolution.member source=viewSet.size receiver=&'static readonly Set<int32, Equality<int32>> type=usize kind=call target="size(parameters=(), arguments=(), return=usize, regions=(\"static\" & \"local\"))"
/// @resolution.place source=viewSet placement="local" lifetime="static" access="immutable"
/// @resolution.access source=viewSet root=viewSet

const viewSetHas = viewSet.has(1);
/// @type.symbol symbol=viewSetHas source=viewSetHas type=boolean
/// @resolution.pattern source=viewSetHas kind=binding target=viewSetHas
/// @resolution.name source=viewSet target=viewSet
/// @resolution.member source=viewSet.has receiver=&'static readonly Set<int32, Equality<int32>> type=<has.Q#1, const has.A#1: "readonly" | "immutable", has#1.'a>(this: WithAccess<&has#1.'a readonly Set<int32, Equality<int32>>, has.A#1>, has.Q#1) => boolean & <has.Q#2, const has.A#2: "readonly" | "immutable", has#2.'a, has#2.'b>(this: WithAccess<&has#2.'a readonly Set<int32, Equality<int32>>, has.A#2>, &has#2.'b immutable has.Q#2) => boolean kind=overload-set targets=[has#1, has#2]
/// @resolution.call source=viewSet.has(1) parameters=(int32) arguments=(provided(1) as int32) return=boolean regions=("static" & "local") kind=symbol target=has#1 receiver=&'static readonly Set<int32, Equality<int32>> instance="Set<int32, Equality<int32>>.<extension#6>.has#1<int32, \"readonly\", \"static\" & \"local\">"
/// @resolution.place source=viewSet placement="local" lifetime="static" access="immutable"
/// @resolution.access source=viewSet root=viewSet
/// @generic.instantiation id="has#1<int32, Equality<int32>, int32, \"readonly\", \"static\" & \"local\">" template=has#1 arguments=(int32, Equality<int32>, int32, "readonly", "static" & "local")
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

    bump(&this): void {
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

    bump(&this): void {
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
    owned.bump<"frame">();
    managed.bump<"managed">();
    borrowed.bump<'a>();
    view.bump<'b>();
}

=== dir ===
class Counter {
/// @type.symbol symbol=Counter type=typeof Counter
/// @definition.class symbol=Counter
/// @definition.field symbol=Counter.count source="count: int32 = 0" key=count type=int32
/// @definition.method symbol=Counter.bump slot=bump type=<Counter.bump.'a>(this: &Counter.bump.'a Counter) => void
/// @definition.method symbol=Counter.total slot=total role=getter type=<Counter.total.'a>(this: &Counter.total.'a readonly Counter) => int32

    count: int32 = 0;
    /// @type.symbol symbol=Counter.count source="count: int32 = 0" type=int32

    get total(&readonly this): int32 {
    /// @generic.template symbol=Counter.total parameters=('a)
    /// @type.symbol symbol=Counter.total type=<Counter.total.'a>(this: &Counter.total.'a readonly Counter) => int32
    /// @type.symbol symbol=Counter.total.this source="&readonly this" type=&Counter.total.'a readonly Counter

        return this.count;
        /// @resolution.member source=this.count receiver=&Counter.total.'a readonly Counter type=int32 kind=field target_receiver=&Counter.total.'a readonly Counter key=count target=Counter.count target_type=int32
        /// @resolution.receiver source=this kind=this declaration=Counter type=&Counter.total.'a readonly Counter
        /// @resolution.place source=this placement=Counter.total.'a lifetime=Counter.total.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.count placement=Counter.total.'a lifetime=Counter.total.'a access="readonly"
        /// @resolution.access source=this.count root=this keys=[count]

    }

    bump(&this): void {
    /// @generic.template symbol=Counter.bump parameters=('a)
    /// @type.symbol symbol=Counter.bump type=<Counter.bump.'a>(this: &Counter.bump.'a Counter) => void
    /// @type.symbol symbol=Counter.bump.this source=&this type=&Counter.bump.'a Counter

        this.count += 1;
        /// @resolution.operator source="this.count += 1" type=int32 operator="+" kind=builtin operands=[this.count as int32 families=(integer), 1 as int32 families=(integer)]
        /// @resolution.receiver source=this kind=this declaration=Counter type=&Counter.bump.'a Counter
        /// @resolution.place source=this placement=Counter.bump.'a lifetime=Counter.bump.'a access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.count kind=place
        /// @resolution.place source=this.count placement=Counter.bump.'a lifetime=Counter.bump.'a access="mutable"
        /// @resolution.assignment source=this.count read="receiver=&Counter.bump.'a Counter, target=field(receiver=&Counter.bump.'a Counter, target=Counter.count, type=int32), type=int32" write="receiver=&Counter.bump.'a Counter, target=field(receiver=&Counter.bump.'a Counter, target=Counter.count, type=int32), type=int32" type=int32
        /// @resolution.access source=this.count root=this keys=[count]

    }
}

declare const owned: ^Counter;
/// @type.symbol symbol=owned source=owned type=^Counter
/// @resolution.pattern source=owned kind=binding target=owned
/// @resolution.name source=Counter target=Counter

declare const managed: Counter;
/// @type.symbol symbol=managed source=managed type=Counter
/// @resolution.pattern source=managed kind=binding target=managed
/// @resolution.name source=Counter target=Counter

declare const borrowed: &Counter;
/// @type.symbol symbol=borrowed source=borrowed type=&'static Counter
/// @resolution.pattern source=borrowed kind=binding target=borrowed
/// @resolution.name source=Counter target=Counter

declare const view: &readonly Counter;
/// @type.symbol symbol=view source=view type=&'static readonly Counter
/// @resolution.pattern source=view kind=binding target=view
/// @resolution.name source=Counter target=Counter

const ownedTotal = owned.total;
/// @type.symbol symbol=ownedTotal source=ownedTotal type=int32
/// @resolution.pattern source=ownedTotal kind=binding target=ownedTotal
/// @resolution.name source=owned target=owned
/// @resolution.member source=owned.total receiver=^Counter type=int32 kind=call target="Counter.total(parameters=(), arguments=(), return=int32, regions=(\"static\" & \"local\"))"
/// @resolution.place source=owned placement="local" lifetime="static" access="immutable"
/// @resolution.access source=owned root=owned
/// @generic.instantiation id="Counter.total<\"static\" & \"local\">" template=Counter.total arguments=("static" & "local")

const managedTotal = managed.total;
/// @type.symbol symbol=managedTotal source=managedTotal type=int32
/// @resolution.pattern source=managedTotal kind=binding target=managedTotal
/// @resolution.name source=managed target=managed
/// @resolution.member source=managed.total receiver=Counter type=int32 kind=call target="Counter.total(parameters=(), arguments=(), return=int32, regions=(\"managed\" & \"local\"))"
/// @resolution.place source=managed placement="local" lifetime="static" access="immutable"
/// @resolution.access source=managed root=managed
/// @generic.instantiation id="Counter.total<\"managed\" & \"local\">" template=Counter.total arguments=("managed" & "local")

const borrowedTotal = borrowed.total;
/// @type.symbol symbol=borrowedTotal source=borrowedTotal type=int32
/// @resolution.pattern source=borrowedTotal kind=binding target=borrowedTotal
/// @resolution.name source=borrowed target=borrowed
/// @resolution.member source=borrowed.total receiver=&'static Counter type=int32 kind=call target="Counter.total(parameters=(), arguments=(), return=int32, regions=(\"static\" & \"local\"))"
/// @resolution.place source=borrowed placement="local" lifetime="static" access="immutable"
/// @resolution.access source=borrowed root=borrowed

const viewTotal = view.total;
/// @type.symbol symbol=viewTotal source=viewTotal type=int32
/// @resolution.pattern source=viewTotal kind=binding target=viewTotal
/// @resolution.name source=view target=view
/// @resolution.member source=view.total receiver=&'static readonly Counter type=int32 kind=call target="Counter.total(parameters=(), arguments=(), return=int32, regions=(\"static\" & \"local\"))"
/// @resolution.place source=view placement="local" lifetime="static" access="immutable"
/// @resolution.access source=view root=view

function mutate(owned: ^Counter, managed: Counter, borrowed: &Counter, view: &readonly Counter): void {
/// @generic.template symbol=mutate parameters=('a, 'b)
/// @type.symbol symbol=mutate type=<mutate.'a, mutate.'b>(^Counter, Counter, &mutate.'a Counter, &mutate.'b readonly Counter) => void
/// @type.symbol symbol=mutate.owned source="owned: ^Counter" type=^Counter
/// @resolution.name source=Counter target=Counter
/// @type.symbol symbol=mutate.managed source="managed: Counter" type=Counter
/// @resolution.name source=Counter target=Counter
/// @type.symbol symbol=mutate.borrowed source="borrowed: &Counter" type=&mutate.'a Counter
/// @resolution.name source=Counter target=Counter
/// @type.symbol symbol=mutate.view source="view: &readonly Counter" type=&mutate.'b readonly Counter
/// @resolution.name source=Counter target=Counter

    owned.bump();
    /// @resolution.name source=owned target=mutate.owned
    /// @resolution.member source=owned.bump receiver=^Counter type=<Counter.bump.'a>(this: &Counter.bump.'a Counter) => void kind=symbol target_receiver=^Counter target=Counter.bump
    /// @resolution.call source=owned.bump() parameters=() return=void regions=("frame" & "local") kind=symbol target=Counter.bump receiver=^Counter adjustments=(borrow(&'frame Counter)) instance="Counter.bump<\"frame\" & \"local\">"
    /// @resolution.place source=owned placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=owned root=mutate.owned
    /// @generic.instantiation id="Counter.bump<\"frame\" & \"local\">" template=Counter.bump arguments=("frame" & "local")

    managed.bump();
    /// @resolution.name source=managed target=mutate.managed
    /// @resolution.member source=managed.bump receiver=Counter type=<Counter.bump.'a>(this: &Counter.bump.'a Counter) => void kind=symbol target_receiver=Counter target=Counter.bump
    /// @resolution.call source=managed.bump() parameters=() return=void regions=("managed" & "local") kind=symbol target=Counter.bump receiver=Counter adjustments=(borrow(&'managed Counter)) instance="Counter.bump<\"managed\" & \"local\">"
    /// @resolution.place source=managed placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=managed root=mutate.managed
    /// @generic.instantiation id="Counter.bump<\"managed\" & \"local\">" template=Counter.bump arguments=("managed" & "local")

    borrowed.bump();
    /// @resolution.name source=borrowed target=mutate.borrowed
    /// @resolution.member source=borrowed.bump receiver=&mutate.'a Counter type=<Counter.bump.'a>(this: &Counter.bump.'a Counter) => void kind=symbol target_receiver=&mutate.'a Counter target=Counter.bump
    /// @resolution.call source=borrowed.bump() parameters=() return=void regions=(mutate.'a) kind=symbol target=Counter.bump receiver=&mutate.'a Counter instance=Counter.bump<mutate.'a>
    /// @resolution.place source=borrowed placement=mutate.'a lifetime=mutate.'a access="mutable"
    /// @resolution.access source=borrowed root=mutate.borrowed
    /// @generic.instantiation id=Counter.bump<mutate.'a> template=Counter.bump arguments=(mutate.'a)

    view.bump();
    /// @resolution.name source=view target=mutate.view
    /// @resolution.member source=view.bump receiver=&mutate.'b readonly Counter type=<Counter.bump.'a>(this: &Counter.bump.'a Counter) => void kind=symbol target_receiver=&mutate.'b readonly Counter target=Counter.bump
    /// @resolution.call source=view.bump() parameters=() return=void regions=(mutate.'b) kind=symbol target=Counter.bump receiver=&mutate.'b readonly Counter instance=Counter.bump<mutate.'b>
    /// @resolution.place source=view placement=mutate.'b lifetime=mutate.'b access="readonly"
    /// @resolution.access source=view root=mutate.view
    /// @generic.instantiation id=Counter.bump<mutate.'b> template=Counter.bump arguments=(mutate.'b)

}
"#,
        r#"
/// @diagnostic.error id=receiver-not-assignable message="receiver type '&'b readonly Counter' is not assignable to the method's 'this' type '&'b Counter'"
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
declare function take<'a>(value: &readonly int32): void;
declare function takeText<'a>(value: &readonly string): void;
declare function make(): int32;

const stored: int32 = 1;

take<"frame">(1 as &'frame readonly int32);
take<"static">(stored as &'static readonly int32);
take<"frame">(make() as &'frame readonly int32);
takeText<"managed">("a" as &'managed readonly string);

=== dir ===
declare function take(value: &readonly int32): void;
/// @generic.template symbol=take parameters=('a)
/// @type.symbol symbol=take source="declare function take(value: &readonly int32): void" type=<take.'a>(&take.'a readonly int32) => void

declare function takeText(value: &readonly string): void;
/// @generic.template symbol=takeText parameters=('a)
/// @type.symbol symbol=takeText source="declare function takeText(value: &readonly string): void" type=<takeText.'a>(&takeText.'a readonly string) => void

declare function make(): int32;
/// @type.symbol symbol=make source="declare function make(): int32" type=() => int32

const stored: int32 = 1;
/// @type.symbol symbol=stored source=stored type=int32
/// @resolution.pattern source=stored kind=binding target=stored
/// @coercion.node source=1 from=1 adjustments=[{ kind: materialize, target: int32 }] origin=implicit

take(1);
/// @resolution.name source=take target=take
/// @resolution.call source=take(1) parameters=(&'frame readonly int32) arguments=(provided(1) as &'frame readonly int32) return=void regions=("frame" & "local") kind=symbol target=take instance="take<\"frame\" & \"local\">"
/// @generic.instantiation id="take<\"frame\" & \"local\">" template=take arguments=("frame" & "local")
/// @coercion.node source=1 from=1 adjustments=[{ kind: materialize, target: int32 }, { kind: borrow, target: &'frame readonly int32 }] origin=implicit

take(stored);
/// @resolution.name source=take target=take
/// @resolution.call source=take(stored) parameters=(&'static readonly int32) arguments=(provided(stored) as &'static readonly int32) return=void regions=("static" & "local") kind=symbol target=take instance="take<\"static\" & \"local\">"
/// @generic.instantiation id="take<\"static\" & \"local\">" template=take arguments=("static" & "local")
/// @resolution.name source=stored target=stored
/// @resolution.place source=stored placement="local" lifetime="static" access="immutable"
/// @resolution.access source=stored root=stored
/// @coercion.node source=stored from=int32 adjustments=[{ kind: borrow, target: &'static readonly int32 }] origin=implicit

take(make());
/// @resolution.name source=take target=take
/// @resolution.call source=take(make()) parameters=(&'frame readonly int32) arguments=(provided(make()) as &'frame readonly int32) return=void regions=("frame" & "local") kind=symbol target=take instance="take<\"frame\" & \"local\">"
/// @resolution.name source=make target=make
/// @resolution.call source=make() parameters=() return=int32 kind=symbol target=make
/// @coercion.node source=make() from=int32 adjustments=[{ kind: borrow, target: &'frame readonly int32 }] origin=implicit

takeText("a");
/// @resolution.name source=takeText target=takeText
/// @resolution.call source="takeText(\"a\")" parameters=(&'managed readonly string) arguments=(provided("a") as &'managed readonly string) return=void regions=("managed" & "local") kind=symbol target=takeText instance="takeText<\"managed\" & \"local\">"
/// @generic.instantiation id="takeText<\"managed\" & \"local\">" template=takeText arguments=("managed" & "local")
/// @coercion.node source="\"a\"" from="a" adjustments=[{ kind: materialize, target: string }, { kind: borrow, target: &'managed readonly string }] origin=implicit
"#,
        r#"

"#,
    );
}
