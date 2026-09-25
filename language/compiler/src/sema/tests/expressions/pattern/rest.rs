use crate::tests::{DirRows, TestSession};

#[test]
fn test_sequence_pattern_rest_binds_array_tail() {
    let session = TestSession::single(
        r#"
declare const values: int32[];

let [head, ...tail] = values;

head satisfies int32;
tail satisfies ^int32[];
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const values: int32[];

let [head, ...tail] = values;

head satisfies int32;
tail satisfies ^int32[];

=== dir ===
declare const values: int32[];
/// @type.symbol symbol=values source=values type=int32[]
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id=Array<int32> template=Array arguments=(int32)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int32>> template=sliceAssumeInit arguments=(MaybeUninit<int32>)
/// @generic.instance id=sliceUninit<MaybeUninit<int32>> template=sliceUninit arguments=(MaybeUninit<int32>)

let [head, ...tail] = values;
/// @resolution.pattern source=[head, ...tail] kind=sequence element=int32 arity=1.. fields=(head) rest=...tail
/// @generic.instantiation id="index#2<int32, \"managed\" & \"local\">" template=index#2 arguments=(int32, "managed" & "local")
/// @generic.instantiation id="rest#2<int32, \"managed\" & \"local\">" template=rest#2 arguments=(int32, "managed" & "local")
/// @generic.instance id="index#2<int32, \"bound0\" & \"local\">" template=index#2 arguments=(int32, "bound0" & "local")
/// @generic.instance id="rest#2<int32, \"bound0\" & \"local\">" template=rest#2 arguments=(int32, "bound0" & "local")
/// @type.symbol symbol=head source=head type=int32
/// @resolution.pattern source=head kind=binding target=head
/// @type.symbol symbol=tail source=tail type=^int32[]
/// @resolution.pattern source=tail kind=binding target=tail
/// @type.node source=values type=int32[]
/// @resolution.name source=values target=values
/// @resolution.access source=values root=values

head satisfies int32;
/// @type.node source="head satisfies int32" type=int32
/// @type.node source=head type=int32
/// @resolution.name source=head target=head
/// @resolution.place source=head placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=head root=head

tail satisfies ^int32[];
/// @type.node source="tail satisfies ^int32[]" type=^int32[]
/// @type.node source=tail type=^int32[]
/// @resolution.name source=tail target=tail
/// @resolution.place source=tail placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=tail root=tail
"#,
    );
}

#[test]
fn test_rest_pattern_must_be_last() {
    let session = TestSession::single(
        r#"
let [...middle, last] = [1, 2, 3];
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let [...middle, last] = [1, 2, 3];

=== dir ===
let [...middle, last] = [1, 2, 3];
/// @resolution.rejected source=[...middle, last]
/// @type.symbol symbol=middle source=middle type=<error>
/// @type.symbol symbol=last source=last type=<error>
/// @type.node source=[1, 2, 3] type=int64[]
/// @resolution.call source=[1, 2, 3] parameters=(^Slice<int64>) arguments=(rest(provided(1) as int64, provided(2) as int64, provided(3) as int64) as int64) return=int64[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int64>
/// @generic.instantiation id=arrayFromOwnedSlice<int64> template=arrayFromOwnedSlice arguments=(int64)
/// @type.node source=1 type=1
/// @type.node source=2 type=2
/// @type.node source=3 type=3
"#,
        r#"
/// @diagnostic.error id=rest-pattern-not-last message="rest pattern must be last"
/// @diagnostic.label line=2 column=6 span="...middle" line_source="let [...middle, last] = [1, 2, 3];"
"#,
    );
}

#[test]
fn test_sequence_pattern_rejects_multiple_rest_patterns() {
    let session = TestSession::single(
        r#"
let [head, ...middle, ...tail] = [1, 2, 3];
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let [head, ...middle, ...tail] = [1, 2, 3];

=== dir ===
let [head, ...middle, ...tail] = [1, 2, 3];
/// @resolution.rejected source=[head, ...middle, ...tail]
/// @type.symbol symbol=head source=head type=<error>
/// @type.symbol symbol=middle source=middle type=<error>
/// @type.symbol symbol=tail source=tail type=<error>
/// @type.node source=[1, 2, 3] type=int64[]
/// @resolution.call source=[1, 2, 3] parameters=(^Slice<int64>) arguments=(rest(provided(1) as int64, provided(2) as int64, provided(3) as int64) as int64) return=int64[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int64>
/// @generic.instantiation id=arrayFromOwnedSlice<int64> template=arrayFromOwnedSlice arguments=(int64)
/// @type.node source=1 type=1
/// @type.node source=2 type=2
/// @type.node source=3 type=3
"#,
        r#"
/// @diagnostic.error id=multiple-rest-patterns message="pattern can contain at most one rest field"
/// @diagnostic.label line=2 column=23 span="...tail" line_source="let [head, ...middle, ...tail] = [1, 2, 3];"
"#,
    );
}
