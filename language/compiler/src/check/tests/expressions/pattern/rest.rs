use crate::tests::{DirRows, TestSession};

#[test]
fn test_sequence_pattern_rest_binds_array_tail() {
    let session = TestSession::single(
        r#"
declare const values: int32[];

let [head, ...tail] = values;

head satisfies int32;
tail satisfies int32[];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const values: int32[];

let [head, ...tail] = values;

head satisfies int32;
tail satisfies int32[];

=== checked ===
declare const values: int32[];
/// @type.symbol symbol=values source=values type=Array<int32>

let [head, ...tail] = values;
/// @type.symbol symbol=head source=head type=int32
/// @type.symbol symbol=tail source=tail type=Array<int32>
/// @resolution.pattern source="[head, ...tail]" kind=sequence sequence=array fields=(head) rest=...tail
/// @resolution.pattern source=head kind=binding target=head
/// @resolution.pattern source=tail kind=binding target=tail
/// @type.node source=values type=Array<int32>
/// @resolution.name source=values target=values

head satisfies int32;
/// @type.node source="head satisfies int32" type=int32
/// @type.node source=head type=int32
/// @resolution.name source=head target=head

tail satisfies int32[];
/// @type.node source="tail satisfies int32[]" type=Array<int32>
/// @type.node source=tail type=Array<int32>
/// @resolution.name source=tail target=tail
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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let [...middle, last] = [1, 2, 3];

=== checked ===
let [...middle, last] = [1, 2, 3];
/// @type.symbol symbol=middle source=middle type=Array<float64>
/// @type.symbol symbol=last source=last type=float64
/// @resolution.pattern source="[...middle, last]" kind=sequence sequence=array fields=(last) rest=...middle
/// @resolution.pattern source=middle kind=binding target=middle
/// @resolution.pattern source=last kind=binding target=last
/// @type.node source=[1, 2, 3] type=Array<float64>
/// @type.node source=1 type=float64
/// @type.node source=2 type=float64
/// @type.node source=3 type=float64
"#,
        r#"
/// @diagnostic.error code=EC430 message="rest pattern must be last"
/// @diagnostic.label line=2 column=6 source="...middle"
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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let [head, ...middle, ...tail] = [1, 2, 3];

=== checked ===
let [head, ...middle, ...tail] = [1, 2, 3];
/// @type.symbol symbol=head source=head type=float64
/// @type.symbol symbol=middle source=middle type=Array<float64>
/// @type.symbol symbol=tail source=tail type=Array<float64>
/// @resolution.pattern source="[head, ...middle, ...tail]" kind=sequence sequence=array fields=(head) rest=...middle
/// @resolution.pattern source=head kind=binding target=head
/// @resolution.pattern source=middle kind=binding target=middle
/// @resolution.pattern source=tail kind=binding target=tail
/// @type.node source=[1, 2, 3] type=Array<float64>
/// @type.node source=1 type=float64
/// @type.node source=2 type=float64
/// @type.node source=3 type=float64
"#,
        r#"
/// @diagnostic.error code=EC431 message="pattern can contain at most one rest field"
/// @diagnostic.label line=2 column=23 source="...tail"
"#,
    );
}
