use crate::tests::{DirRows, TestSession};

#[test]
fn test_scalar_markers_admit_every_width() {
    let session = TestSession::single(
        r#"
import { Numeric } from "destack:math";

struct Vector<T: Numeric> {
    x: T;
}

const ints = Vector { x: 1 as int32 };
const floats = Vector { x: 1.5 as float32 };
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Numeric } from "destack:math";

struct Vector<T: Numeric> {
    x: T;
}

const ints: Vector<int32> = Vector<int32> { x: 1 as int32 };
const floats: Vector<float32> = Vector<float32> { x: 1.5 as float32 };

=== checked ===
import { Numeric } from "destack:math";

struct Vector<T: Numeric> {
/// @generic.template symbol=Vector parameters=(T: math.scalar.Numeric)
/// @type.symbol symbol=Vector type=Vector
/// @definition.struct symbol=Vector template=(T: math.scalar.Numeric)
/// @definition.field symbol=Vector.x source="x: T" key=x type=T
/// @type.symbol symbol=Vector.T source="T: Numeric" type=T
/// @resolution.name source=Numeric target=math.scalar.Numeric

    x: T;
    /// @type.symbol symbol=Vector.x source="x: T" type=T
    /// @resolution.name source=T target=Vector.T

}

const ints = Vector { x: 1 as int32 };
/// @type.symbol symbol=ints source=ints type=Vector<int32>
/// @resolution.name source=Vector target=Vector

const floats = Vector { x: 1.5 as float32 };
/// @type.symbol symbol=floats source=floats type=Vector<float32>
/// @resolution.name source=Vector target=Vector

/// @generic.instance id=Vector<float32> template=Vector arguments=(float32)
/// @generic.instance id=Vector<int32> template=Vector arguments=(int32)
"#,
        r#""#,
    );
}

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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Index<T: int> {
    value: T;
}

const wide: Index<int64> = Index<int64> { value: 1 as int64 };
const narrow: Index<int32> = Index<int32> { value: 1 as int32 };

=== checked ===
struct Index<T: int> {
/// @generic.template symbol=Index parameters=(T: int64)
/// @type.symbol symbol=Index type=Index
/// @definition.struct symbol=Index template=(T: int64)
/// @definition.field symbol=Index.value source="value: T" key=value type=T
/// @type.symbol symbol=Index.T source="T: int" type=T

    value: T;
    /// @type.symbol symbol=Index.value source="value: T" type=T
    /// @resolution.name source=T target=Index.T

}

const wide = Index { value: 1 as int64 };
/// @type.symbol symbol=wide source=wide type=Index<int64>
/// @resolution.name source=Index target=Index

const narrow = Index { value: 1 as int32 };
/// @type.symbol symbol=narrow source=narrow type=Index<int32>
/// @resolution.name source=Index target=Index

/// @generic.instance id=Index<int32> template=Index arguments=(int32)
/// @generic.instance id=Index<int64> template=Index arguments=(int64)
"#,
        r#"
/// @diagnostic.error code=EC201 message="type 'int32' does not satisfy 'int64'"
/// @diagnostic.label line=7 column=16 span="Index" line_source="const narrow = Index { value: 1 as int32 };"
"#,
    );
}

#[test]
fn test_comptime_arithmetic_keeps_the_operand_type() {
    let session = TestSession::single(
        r#"
struct Tensor<comptime Rank: int> {
    rank: usize;
}

function shrink<comptime Rank: int>(tensor: Tensor<Rank>): Tensor<Rank - 1> {
    Tensor { rank: 0 }
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Tensor<comptime Rank: int> {
    rank: usize;
}

function shrink<comptime Rank: int>(tensor: Tensor<Rank>): Tensor<Rank - 1> {
    Tensor { rank: 0 }
}

=== checked ===
struct Tensor<comptime Rank: int> {
/// @generic.template symbol=Tensor parameters=(comptime Rank#1: int64)
/// @type.symbol symbol=Tensor type=Tensor
/// @definition.struct symbol=Tensor template=(comptime Rank#1: int64)
/// @definition.field symbol=Tensor.rank source="rank: usize" key=rank type=usize
/// @type.symbol symbol=Tensor.Rank source="comptime Rank: int" type=Rank#1

    rank: usize;
    /// @type.symbol symbol=Tensor.rank source="rank: usize" type=usize

}

function shrink<comptime Rank: int>(tensor: Tensor<Rank>): Tensor<Rank - 1> {
/// @generic.template symbol=shrink parameters=(comptime Rank#2: int64)
/// @type.symbol symbol=shrink type=<comptime Rank#2: int64>(Tensor<Rank#2>) => Tensor<Rank#2 - 1>
/// @type.symbol symbol=shrink.Rank source="comptime Rank: int" type=Rank#2
/// @type.symbol symbol=shrink.tensor source="tensor: Tensor<Rank>" type=Tensor<Rank#2>
/// @resolution.name source=Tensor target=Tensor
/// @resolution.name source=Rank target=shrink.Rank
/// @resolution.name source=Tensor target=Tensor
/// @resolution.name source=Rank target=shrink.Rank

    Tensor { rank: 0 }
    /// @resolution.name source=Tensor target=Tensor

}

/// @generic.instance id="Tensor<Rank#2 - 1>" template=Tensor arguments=(Rank#2 - 1)
/// @generic.instance id=Tensor<Rank#2> template=Tensor arguments=(Rank#2)
"#,
        r#""#,
    );
}
