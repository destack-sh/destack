use crate::tests::{DirRows, TestSession};

#[test]
fn test_object_assignment_pattern_resolves_targets() {
    let session = TestSession::single(
        r#"
let x: int32 = 0;
let label: string = "";
declare const point: { x: int32; y: string };

({ x, y: label } = point);
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let x: int32 = 0;
let label: string = "";
declare const point: { x: int32; y: string };

({ x, y: label } = point);

=== dir ===
let x: int32 = 0;
/// @type.symbol symbol=x#1 source=x type=int32
/// @resolution.pattern source=x kind=binding target=x#1
/// @type.node source=0 type=0

let label: string = "";
/// @type.symbol symbol=label source=label type=string
/// @resolution.pattern source=label kind=binding target=label
/// @type.node source="\"\"" type=""

declare const point: { x: int32; y: string };
/// @type.symbol symbol=point source=point type={ x: int32; y: string }
/// @resolution.pattern source=point kind=binding target=point
/// @type.symbol symbol=x#2 source="x: int32" type=int32
/// @type.symbol symbol=y source="y: string" type=string

({ x, y: label } = point);
/// @type.node source="{ x, y: label } = point" type={ x: int32; y: string }
/// @resolution.pattern.assign source={ x, y: label } kind=object fields={ x, y: label }
/// @resolution.access source={ x, y: label } root=point
/// @type.node source=x type=int32
/// @resolution.name source=x target=x#1
/// @resolution.pattern.assign source=x kind=place
/// @resolution.place source=x placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=x root=point keys=[x]
/// @resolution.access source=x root=x#1
/// @resolution.assignment source=x write=binding(x#1) type=int32
/// @resolution.access source="y: label" root=point keys=[y]
/// @type.node source=label type=string
/// @resolution.name source=label target=label
/// @resolution.pattern.assign source=label kind=place
/// @resolution.place source=label placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=label root=label
/// @resolution.assignment source=label write=binding(label) type=string
/// @type.node source=point type={ x: int32; y: string }
/// @resolution.name source=point target=point
/// @resolution.place source=point placement="local" lifetime="static" access="immutable"
/// @resolution.access source=point root=point
"#,
    );
}

#[test]
fn test_sequence_assignment_pattern_resolves_targets() {
    let session = TestSession::single(
        r#"
let first: int32 = 0;
let last: int32 = 0;
declare const values: [int32; 3];

[first, , last] = values;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let first: int32 = 0;
let last: int32 = 0;
declare const values: [int32; 3];

[first, , last] = values;

=== dir ===
let first: int32 = 0;
/// @type.symbol symbol=first source=first type=int32
/// @resolution.pattern source=first kind=binding target=first
/// @type.node source=0 type=0

let last: int32 = 0;
/// @type.symbol symbol=last source=last type=int32
/// @resolution.pattern source=last kind=binding target=last
/// @type.node source=0 type=0

declare const values: [int32; 3];
/// @type.symbol symbol=values source=values type=FixedArray<int32, 3>
/// @resolution.pattern source=values kind=binding target=values

[first, , last] = values;
/// @type.node source="[first, , last] = values" type=FixedArray<int32, 3>
/// @resolution.pattern.assign source=[first, , last] kind=sequence element=int32 arity=3 fields=(first, last)
/// @resolution.access source=[first, , last] root=values
/// @generic.instantiation id="index#2<int32, 3, \"frame\" & \"local\">" template=index#2 arguments=(int32, 3, "frame" & "local")
/// @generic.instance id="FixedArray<int32, 3>" template=FixedArray arguments=(int32, 3)
/// @generic.instance id="index#2<int32, 3, \"bound0\" & \"local\">" template=index#2 arguments=(int32, 3, "bound0" & "local")
/// @type.node source=first type=int32
/// @resolution.name source=first target=first
/// @resolution.pattern.assign source=first kind=place
/// @resolution.place source=first placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=first root=first
/// @resolution.assignment source=first write=binding(first) type=int32
/// @type.node source=last type=int32
/// @resolution.name source=last target=last
/// @resolution.pattern.assign source=last kind=place
/// @resolution.place source=last placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=last root=last
/// @resolution.assignment source=last write=binding(last) type=int32
/// @type.node source=values type=FixedArray<int32, 3>
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="local" lifetime="static" access="immutable"
/// @resolution.access source=values root=values
"#,
    );
}

#[test]
fn test_object_assignment_pattern_resolves_rest_target() {
    let session = TestSession::single(
        r#"
let name: string = "";
let rest: { age: int32; active: boolean } = { age: 0, active: false };
declare const user: { name: string; age: int32; active: boolean };

({ name, ...rest } = user);
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let name: string = "";
let rest: { age: int32; active: boolean } = { age: 0, active: false };
declare const user: { name: string; age: int32; active: boolean };

({ name, ...rest } = user);

=== dir ===
let name: string = "";
/// @type.symbol symbol=name#1 source=name type=string
/// @resolution.pattern source=name kind=binding target=name#1
/// @type.node source="\"\"" type=""

let rest: { age: int32; active: boolean } = { age: 0, active: false };
/// @type.symbol symbol=rest source=rest type={ age: int32; active: boolean }
/// @resolution.pattern source=rest kind=binding target=rest
/// @type.symbol symbol=age#1 source="age: int32" type=int32
/// @type.symbol symbol=active#1 source="active: boolean" type=boolean
/// @type.node source={ age: 0, active: false } type={ age: int32; active: boolean }
/// @type.node source=0 type=0
/// @type.node source=false type=false

declare const user: { name: string; age: int32; active: boolean };
/// @type.symbol symbol=user source=user type={ name: string; age: int32; active: boolean }
/// @resolution.pattern source=user kind=binding target=user
/// @type.symbol symbol=name#2 source="name: string" type=string
/// @type.symbol symbol=age#2 source="age: int32" type=int32
/// @type.symbol symbol=active#2 source="active: boolean" type=boolean

({ name, ...rest } = user);
/// @type.node source="{ name, ...rest } = user" type={ name: string; age: int32; active: boolean }
/// @resolution.pattern.assign source={ name, ...rest } kind=object fields={ name } rest=...rest
/// @resolution.access source={ name, ...rest } root=user
/// @type.node source=name type=string
/// @resolution.name source=name target=name#1
/// @resolution.pattern.assign source=name kind=place
/// @resolution.place source=name placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=name root=name#1
/// @resolution.access source=name root=user keys=[name]
/// @resolution.assignment source=name write=binding(name#1) type=string
/// @type.node source=rest type={ age: int32; active: boolean }
/// @resolution.name source=rest target=rest
/// @resolution.pattern.assign source=rest kind=place
/// @resolution.place source=rest placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=rest root=rest
/// @resolution.assignment source=rest write=binding(rest) type={ age: int32; active: boolean }
/// @type.node source=user type={ name: string; age: int32; active: boolean }
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="local" lifetime="static" access="immutable"
/// @resolution.access source=user root=user
"#,
    );
}

#[test]
fn test_sequence_assignment_pattern_resolves_rest_target() {
    let session = TestSession::single(
        r#"
let head: int32 = 0;
let tail: int32[] = [];
declare const values: int32[];

[head, ...tail] = values;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let head: int32 = 0;
let tail: int32[] = [];
declare const values: int32[];

[head, ...tail] = values;

=== dir ===
let head: int32 = 0;
/// @type.symbol symbol=head source=head type=int32
/// @resolution.pattern source=head kind=binding target=head
/// @type.node source=0 type=0

let tail: int32[] = [];
/// @type.symbol symbol=tail source=tail type=int32[]
/// @resolution.pattern source=tail kind=binding target=tail
/// @generic.instance id=Array<int32> template=Array arguments=(int32)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int32>> template=sliceAssumeInit arguments=(MaybeUninit<int32>)
/// @generic.instance id=sliceUninit<MaybeUninit<int32>> template=sliceUninit arguments=(MaybeUninit<int32>)
/// @type.node source=[] type=int32[]
/// @resolution.call source=[] parameters=(^Slice<int32>) arguments=(rest() as int32) return=int32[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int32>
/// @generic.instantiation id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
/// @generic.instance id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)

declare const values: int32[];
/// @type.symbol symbol=values source=values type=int32[]
/// @resolution.pattern source=values kind=binding target=values

[head, ...tail] = values;
/// @type.node source="[head, ...tail] = values" type=int32[]
/// @resolution.pattern.assign source=[head, ...tail] kind=sequence element=int32 arity=1.. fields=(head) rest=...tail
/// @resolution.access source=[head, ...tail] root=values
/// @generic.instantiation id="index#2<int32, \"managed\" & \"local\">" template=index#2 arguments=(int32, "managed" & "local")
/// @generic.instantiation id="rest#2<int32, \"managed\" & \"local\">" template=rest#2 arguments=(int32, "managed" & "local")
/// @generic.instance id="index#2<int32, \"bound0\" & \"local\">" template=index#2 arguments=(int32, "bound0" & "local")
/// @generic.instance id="rest#2<int32, \"bound0\" & \"local\">" template=rest#2 arguments=(int32, "bound0" & "local")
/// @type.node source=head type=int32
/// @resolution.name source=head target=head
/// @resolution.pattern.assign source=head kind=place
/// @resolution.place source=head placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=head root=head
/// @resolution.assignment source=head write=binding(head) type=int32
/// @type.node source=tail type=int32[]
/// @resolution.name source=tail target=tail
/// @resolution.pattern.assign source=tail kind=place
/// @resolution.place source=tail placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=tail root=tail
/// @resolution.assignment source=tail write=binding(tail) type=int32[]
/// @type.node source=values type=int32[]
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="local" lifetime="static" access="immutable"
/// @resolution.access source=values root=values
"#,
    );
}

#[test]
fn test_nested_assignment_pattern_resolves_leaf_targets() {
    let session = TestSession::single(
        r#"
let x: int32 = 0;
let label: string = "";
declare const packet: { point: { x: int32 }; meta: (string,) };

({
    point: { x },
    meta: (label,),
} = packet);
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let x: int32 = 0;
let label: string = "";
declare const packet: { point: { x: int32 }; meta: (string,) };

({
    point: { x },
    meta: (label,),
} = packet);

=== dir ===
let x: int32 = 0;
/// @type.symbol symbol=x#1 source=x type=int32
/// @resolution.pattern source=x kind=binding target=x#1
/// @type.node source=0 type=0

let label: string = "";
/// @type.symbol symbol=label source=label type=string
/// @resolution.pattern source=label kind=binding target=label
/// @type.node source="\"\"" type=""

declare const packet: { point: { x: int32 }; meta: (string,) };
/// @type.symbol symbol=packet source=packet type={ point: { x: int32 }; meta: (string,) }
/// @resolution.pattern source=packet kind=binding target=packet
/// @type.symbol symbol=point source="point: { x: int32 }" type={ x: int32 }
/// @type.symbol symbol=x#2 source="x: int32" type=int32
/// @type.symbol symbol=meta source="meta: (string,)" type=(string,)

({
/// @type.node type={ point: { x: int32 }; meta: (string,) }
/// @resolution.pattern.assign kind=object fields={ point: assignment_pattern, meta: assignment_pattern }
/// @resolution.access root=packet

    point: { x },
    /// @resolution.access source="point: { x }" root=packet keys=[point]
    /// @resolution.pattern.assign source={ x } kind=object fields={ x }
    /// @type.node source=x type=int32
    /// @resolution.name source=x target=x#1
    /// @resolution.pattern.assign source=x kind=place
    /// @resolution.place source=x placement="local" lifetime="static" access="exclusive"
    /// @resolution.access source=x root=x#1
    /// @resolution.assignment source=x write=binding(x#1) type=int32

    meta: (label,),
    /// @resolution.access source="meta: (label,)" root=packet keys=[meta]
    /// @resolution.pattern.assign source=(label,) kind=tuple fields=(label)
    /// @type.node source=label type=string
    /// @resolution.name source=label target=label
    /// @resolution.pattern.assign source=label kind=place
    /// @resolution.place source=label placement="local" lifetime="static" access="exclusive"
    /// @resolution.access source=label root=label
    /// @resolution.assignment source=label write=binding(label) type=string

} = packet);
/// @type.node source=packet type={ point: { x: int32 }; meta: (string,) }
/// @resolution.name source=packet target=packet
/// @resolution.place source=packet placement="local" lifetime="static" access="immutable"
/// @resolution.access source=packet root=packet
"#,
    );
}

#[test]
fn test_assignment_pattern_default_resolves_inside_object_and_sequence() {
    let session = TestSession::single(
        r#"
let count: int32 = 0;
let label: string = "";
declare const packet: { count?: int32; labels: (string | undefined,) };

({
    count = 1,
    labels: (label = "missing",),
} = packet);
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let count: int32 = 0;
let label: string = "";
declare const packet: { count?: int32; labels: (string | undefined,) };

({
    count = 1,
    labels: (label = "missing",),
} = packet);

=== dir ===
let count: int32 = 0;
/// @type.symbol symbol=count#1 source=count type=int32
/// @resolution.pattern source=count kind=binding target=count#1
/// @type.node source=0 type=0

let label: string = "";
/// @type.symbol symbol=label source=label type=string
/// @resolution.pattern source=label kind=binding target=label
/// @type.node source="\"\"" type=""

declare const packet: { count?: int32; labels: (string | undefined,) };
/// @type.symbol symbol=packet source=packet type={ count?: int32; labels: (string | undefined,) }
/// @resolution.pattern source=packet kind=binding target=packet
/// @type.symbol symbol=count#2 source="count?: int32" type=int32
/// @type.symbol symbol=labels source="labels: (string | undefined,)" type=(string | undefined,)

({
/// @type.node type={ count?: int32; labels: (string | undefined,) }
/// @resolution.pattern.assign kind=object fields={ count, labels: assignment_pattern }
/// @resolution.access root=packet

    count = 1,
    /// @type.node source=count type=int32
    /// @resolution.name source=count target=count#1
    /// @resolution.pattern.assign source="count = 1" kind=default pattern=count value=expression
    /// @resolution.access source="count = 1" root=packet keys=[count]
    /// @resolution.pattern.assign source=count kind=place
    /// @resolution.place source=count placement="local" lifetime="static" access="exclusive"
    /// @resolution.access source=count root=count#1
    /// @resolution.assignment source=count write=binding(count#1) type=int32
    /// @type.node source=1 type=1

    labels: (label = "missing",),
    /// @resolution.access source="labels: (label = \"missing\",)" root=packet keys=[labels]
    /// @resolution.pattern.assign source=(label = "missing",) kind=tuple fields=(label)
    /// @type.node source=label type=string
    /// @resolution.name source=label target=label
    /// @resolution.pattern.assign source="label = \"missing\"" kind=default pattern=label value=expression
    /// @resolution.pattern.assign source=label kind=place
    /// @resolution.place source=label placement="local" lifetime="static" access="exclusive"
    /// @resolution.access source=label root=label
    /// @resolution.assignment source=label write=binding(label) type=string
    /// @type.node source="\"missing\"" type="missing"

} = packet);
/// @type.node source=packet type={ count?: int32; labels: (string | undefined,) }
/// @resolution.name source=packet target=packet
/// @resolution.place source=packet placement="local" lifetime="static" access="immutable"
/// @resolution.access source=packet root=packet
"#,
    );
}

#[test]
fn test_computed_assignment_pattern_accepts_static_key() {
    let session = TestSession::single(
        r#"
let value: int32 = 0;
declare const point: { x: int32 };

({ ["x"]: value } = point);
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let value: int32 = 0;
declare const point: { x: int32 };

({ ["x"]: value } = point);

=== dir ===
let value: int32 = 0;
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=0 type=0

declare const point: { x: int32 };
/// @type.symbol symbol=point source=point type={ x: int32 }
/// @resolution.pattern source=point kind=binding target=point
/// @type.symbol symbol=x source="x: int32" type=int32

({ ["x"]: value } = point);
/// @type.node source="{ [\"x\"]: value } = point" type={ x: int32 }
/// @resolution.pattern.assign source={ ["x"]: value } kind=object fields={ x: value }
/// @resolution.access source={ ["x"]: value } root=point
/// @resolution.access source="[\"x\"]: value" root=point keys=[x]
/// @type.node source="\"x\"" type="x"
/// @type.node source=value type=int32
/// @resolution.name source=value target=value
/// @resolution.pattern.assign source=value kind=place
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=value root=value
/// @resolution.assignment source=value write=binding(value) type=int32
/// @type.node source=point type={ x: int32 }
/// @resolution.name source=point target=point
/// @resolution.place source=point placement="local" lifetime="static" access="immutable"
/// @resolution.access source=point root=point
"#,
    );
}

#[test]
fn test_computed_assignment_pattern_rejects_partial_index_signature_key() {
    let session = TestSession::single(
        r#"
declare const key: string;
let value: int32 = 0;
declare const bag: { [key: string]: int32 };

({ [key]: value } = bag);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const key: string;
let value: int32 = 0;
declare const bag: { [key: string]: int32 };

({ [key]: value } = bag);

=== dir ===
declare const key: string;
/// @type.symbol symbol=key source=key type=string
/// @resolution.pattern source=key kind=binding target=key

let value: int32 = 0;
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=0 type=0

declare const bag: { [key: string]: int32 };
/// @type.symbol symbol=bag source=bag type={ [key: string]: int32 }
/// @resolution.pattern source=bag kind=binding target=bag

({ [key]: value } = bag);
/// @type.node source="{ [key]: value } = bag" type={ [key: string]: int32 }
/// @resolution.pattern.assign source={ [key]: value } kind=object fields={ subscript(member(receiver={ [key: string]: int32 }, target=index(string), type=int32 | undefined), int32 | undefined): value }
/// @resolution.access source={ [key]: value } root=bag
/// @type.node source=key type=string
/// @resolution.name source=key target=key
/// @resolution.place source=key placement="local" lifetime="static" access="immutable"
/// @resolution.access source=key root=key
/// @type.node source=value type=int32
/// @resolution.name source=value target=value
/// @resolution.pattern.assign source=value kind=place
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=value root=value
/// @resolution.assignment source=value write=binding(value) type=int32
/// @type.node source=bag type={ [key: string]: int32 }
/// @resolution.name source=bag target=bag
/// @resolution.place source=bag placement="local" lifetime="static" access="immutable"
/// @resolution.access source=bag root=bag
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'int32 | undefined' is not assignable to type 'int32'"
/// @diagnostic.label line=6 column=11 span="value" line_source="({ [key]: value } = bag);"
/// @diagnostic.note message="expected 'int32', found 'undefined'"
"#,
    );
}

#[test]
fn test_computed_assignment_pattern_rejects_unbounded_key_for_finite_shape() {
    let session = TestSession::single(
        r#"
declare const key: string;
let value: int32 = 0;
declare const point: { x: int32 };

({ [key]: value } = point);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const key: string;
let value: int32 = 0;
declare const point: { x: int32 };

({ [key]: value } = point);

=== dir ===
declare const key: string;
/// @type.symbol symbol=key source=key type=string
/// @resolution.pattern source=key kind=binding target=key

let value: int32 = 0;
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=0 type=0

declare const point: { x: int32 };
/// @type.symbol symbol=point source=point type={ x: int32 }
/// @resolution.pattern source=point kind=binding target=point
/// @type.symbol symbol=x source="x: int32" type=int32

({ [key]: value } = point);
/// @type.node source="{ [key]: value } = point" type=<error>
/// @resolution.rejected source={ [key]: value }
/// @resolution.access source={ [key]: value } root=point
/// @type.node source=key type=string
/// @resolution.name source=key target=key
/// @resolution.place source=key placement="local" lifetime="static" access="immutable"
/// @resolution.access source=key root=key
/// @resolution.name source=value target=value
/// @type.node source=point type={ x: int32 }
/// @resolution.name source=point target=point
/// @resolution.place source=point placement="local" lifetime="static" access="immutable"
/// @resolution.access source=point root=point
"#,
        r#"
/// @diagnostic.error id=computed-pattern-key-not-valid message="computed pattern key is not valid for the source type"
/// @diagnostic.label line=6 column=5 span="key" line_source="({ [key]: value } = point);"
"#,
    );
}
