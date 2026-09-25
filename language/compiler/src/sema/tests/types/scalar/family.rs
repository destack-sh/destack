use crate::tests::{DirRows, TestSession};

/// A scalar family bound admits every width in that family.
#[test]
fn test_scalar_markers_admit_every_width() {
    let session = TestSession::single(
        r#"
import { Numeric } from "tspp:math";

struct Vector<T: Numeric> {
    x: T;
}

const ints = Vector { x: 1 as int32 };
const floats = Vector { x: 1.5 as float32 };
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { Numeric } from "tspp:math";

struct Vector<out T: Numeric> {
    x: T;
}

const ints: Vector<int32> = Vector<int32> { x: 1 as int32 };
const floats: Vector<float32> = Vector<float32> { x: 1.5 as float32 };

=== dir ===
import { Numeric } from "tspp:math";

struct Vector<T: Numeric> {
/// @generic.template symbol=Vector parameters=(out T: Numeric)
/// @type.symbol symbol=Vector type=Vector
/// @definition.struct symbol=Vector template=(out T: Numeric)
/// @definition.field symbol=Vector.x source="x: T" key=x type=T
/// @type.symbol symbol=Vector.T source="T: Numeric" type=T
/// @resolution.name source=Numeric target=Numeric

    x: T;
    /// @type.symbol symbol=Vector.x source="x: T" type=T
    /// @resolution.name source=T target=Vector.T

}

const ints = Vector { x: 1 as int32 };
/// @type.symbol symbol=ints source=ints type=Vector<int32>
/// @resolution.pattern source=ints kind=binding target=ints
/// @generic.instance id=Vector<int32> template=Vector arguments=(int32)
/// @resolution.name source=Vector target=Vector

const floats = Vector { x: 1.5 as float32 };
/// @type.symbol symbol=floats source=floats type=Vector<float32>
/// @resolution.pattern source=floats kind=binding target=floats
/// @generic.instance id=Vector<float32> template=Vector arguments=(float32)
/// @resolution.name source=Vector target=Vector
"#,
    );
}

/// A sized scalar alias bound admits only its own width.
#[test]
fn test_scalar_aliases_admit_only_their_sized_type() {
    let session = TestSession::single(
        r#"
struct Index<T: int> {
    value: T;
}

const wide = Index { value: 1 as int64 };
const narrow = Index { value: 1 as int32 };
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
struct Index<out T: int> {
    value: T;
}

const wide: Index<int64> = Index<int64> { value: 1 as int64 };
const narrow: Index<int32> = Index<int32> { value: 1 as int32 };

=== dir ===
struct Index<T: int> {
/// @generic.template symbol=Index parameters=(out T: int64)
/// @type.symbol symbol=Index type=Index
/// @definition.struct symbol=Index template=(out T: int64)
/// @definition.field symbol=Index.value source="value: T" key=value type=T
/// @type.symbol symbol=Index.T source="T: int" type=T

    value: T;
    /// @type.symbol symbol=Index.value source="value: T" type=T
    /// @resolution.name source=T target=Index.T

}

const wide = Index { value: 1 as int64 };
/// @type.symbol symbol=wide source=wide type=Index<int64>
/// @resolution.pattern source=wide kind=binding target=wide
/// @resolution.name source=Index target=Index

const narrow = Index { value: 1 as int32 };
/// @type.symbol symbol=narrow source=narrow type=Index<int32>
/// @resolution.pattern source=narrow kind=binding target=narrow
/// @resolution.name source=Index target=Index
"#,
        r#"
/// @diagnostic.error id=constraint-not-satisfied message="type 'int32' does not satisfy 'int64'"
/// @diagnostic.label line=7 column=16 span="Index" line_source="const narrow = Index { value: 1 as int32 };"
/// @diagnostic.related line=2 column=14 span="T" line_source="struct Index<T: int> {" message="required by this bound on 'T'"
/// @diagnostic.error id=constraint-not-satisfied message="type 'int32' does not satisfy 'int64'"
/// @diagnostic.label line=7 column=16 span="Index { value: 1 as int32 }" line_source="const narrow = Index { value: 1 as int32 };"
/// @diagnostic.related line=2 column=14 span="T" line_source="struct Index<T: int> {" message="required by this bound on 'T'"
"#,
    );
}

/// An integer literal outside its annotated width reports a diagnostic.
#[test]
fn test_integer_literals_fit_their_annotated_width() {
    let session = TestSession::single(
        r#"
const fits: int8 = 100;
const overflows: int8 = 300;

type Pair = { 0: string; 1: string };
declare const key: keyof Pair;
const index: usize = key;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_node_types(),
        r#"
=== annotated ===
const fits: int8 = 100;
const overflows: int8 = 300;

type Pair = { 0: string; 1: string };
declare const key: 0 | 1;
const index: usize = key as usize;

=== dir ===
const fits: int8 = 100;
/// @type.symbol symbol=fits source=fits type=int8
/// @resolution.pattern source=fits kind=binding target=fits
/// @type.node source=100 type=100

const overflows: int8 = 300;
/// @type.symbol symbol=overflows source=overflows type=int8
/// @resolution.pattern source=overflows kind=binding target=overflows
/// @type.node source=300 type=300

type Pair = { 0: string; 1: string };
/// @type.symbol symbol=Pair source="type Pair = { 0: string; 1: string }" type={ 0: string; 1: string }
/// @definition.type symbol=Pair source="type Pair = { 0: string; 1: string }" value={ 0: string; 1: string }
/// @type.symbol symbol=Pair.symbol4 source="0: string" type=string
/// @type.symbol symbol=Pair.symbol6 source="1: string" type=string

declare const key: keyof Pair;
/// @type.symbol symbol=key source=key type=0 | 1
/// @resolution.pattern source=key kind=binding target=key
/// @resolution.name source=Pair target=Pair

const index: usize = key;
/// @type.symbol symbol=index source=index type=usize
/// @resolution.pattern source=index kind=binding target=index
/// @resolution.name source=key target=key
/// @resolution.place source=key placement="local" lifetime="static" access="immutable"
/// @resolution.access source=key root=key
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '300' is not assignable to type 'int8'"
/// @diagnostic.label line=3 column=25 span="300" line_source="const overflows: int8 = 300;"
/// @diagnostic.related line=3 column=18 span="int8" line_source="const overflows: int8 = 300;" message="expected due to this annotation"
"#,
    );
}

/// Const arithmetic keeps the type of its operands.
#[test]
fn test_const_arithmetic_keeps_the_operand_type() {
    let session = TestSession::single(
        r#"
struct Tensor<const Rank: int> {
    slots: [uint8; Rank];
}

function shrink<const Rank: int>(tensor: Tensor<Rank>): Tensor<Rank - 1> {
    Tensor { slots: [0; Rank - 1] }
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
struct Tensor<const Rank: int> {
    slots: [uint8; Rank];
}

function shrink<const Rank: int>(tensor: Tensor<Rank>): Tensor<Rank - 1> {
    Tensor { slots: [0; Rank - 1] }
}

=== dir ===
struct Tensor<const Rank: int> {
/// @generic.template symbol=Tensor parameters=(const Rank#1: int64)
/// @type.symbol symbol=Tensor type=Tensor
/// @definition.struct symbol=Tensor template=(const Rank#1: int64)
/// @definition.field symbol=Tensor.slots source="slots: [uint8; Rank]" key=slots type=FixedArray<uint8, Rank#1>
/// @type.symbol symbol=Tensor.Rank source="const Rank: int" type=Rank#1

    slots: [uint8; Rank];
    /// @type.symbol symbol=Tensor.slots source="slots: [uint8; Rank]" type=FixedArray<uint8, Rank#1>
    /// @resolution.name source=Rank target=Tensor.Rank

}

function shrink<const Rank: int>(tensor: Tensor<Rank>): Tensor<Rank - 1> {
/// @generic.template symbol=shrink parameters=(const Rank#2: int64)
/// @type.symbol symbol=shrink type=<const Rank#2: int64>(Tensor<Rank#2>) => Tensor<Rank#2 - 1>
/// @generic.instance id="Tensor<Rank#2 - 1>" template=Tensor arguments=(Rank#2 - 1)
/// @generic.instance id=Tensor<Rank#2> template=Tensor arguments=(Rank#2)
/// @type.symbol symbol=shrink.Rank source="const Rank: int" type=Rank#2
/// @type.symbol symbol=shrink.tensor source="tensor: Tensor<Rank>" type=Tensor<Rank#2>
/// @resolution.name source=Tensor target=Tensor
/// @resolution.name source=Rank target=shrink.Rank
/// @resolution.name source=Tensor target=Tensor
/// @resolution.name source=Rank target=shrink.Rank

    Tensor { slots: [0; Rank - 1] }
    /// @resolution.name source=Tensor target=Tensor
    /// @resolution.name source=Rank target=shrink.Rank

}
"#,
    );
}

/// Reject the removed u-prefixed integer width names.
#[test]
fn test_reject_u_prefixed_width_names() {
    let session = TestSession::single(
        r#"
declare const value: u32;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const value: u32;

=== dir ===
declare const value: u32;
/// @type.symbol symbol=value source=value type=<error>
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.unresolved source=u32 path=u32
"#,
        r#"
/// @diagnostic.error id=unresolved-reference message="cannot find 'u32'"
/// @diagnostic.label line=2 column=22 span="u32" line_source="declare const value: u32;"
"#,
    );
}
