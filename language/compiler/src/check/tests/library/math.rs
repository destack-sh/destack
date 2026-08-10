use crate::tests::{DirRows, TestSession};

#[test]
fn test_unsigned_preserves_integer_width() {
    let session = TestSession::single(
        r#"
import { Unsigned } from "destack:math";

type SignedFixed = Unsigned<int37>;
type UnsignedFixed = Unsigned<uint37>;
type SignedPointer = Unsigned<isize>;
type UnsignedPointer = Unsigned<usize>;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Unsigned } from "destack:math";

type SignedFixed = Unsigned<int37>;
type UnsignedFixed = Unsigned<uint37>;
type SignedPointer = Unsigned<isize>;
type UnsignedPointer = Unsigned<usize>;

=== checked ===
import { Unsigned } from "destack:math";

type SignedFixed = Unsigned<int37>;
/// @type.symbol symbol=SignedFixed source="type SignedFixed = Unsigned<int37>" type=math.integer.Unsigned<int37> reduced=uint37
/// @definition.type symbol=SignedFixed source="type SignedFixed = Unsigned<int37>" value=math.integer.Unsigned<int37> reduced=uint37
/// @resolution.name source=Unsigned target=math.integer.Unsigned

type UnsignedFixed = Unsigned<uint37>;
/// @type.symbol symbol=UnsignedFixed source="type UnsignedFixed = Unsigned<uint37>" type=math.integer.Unsigned<uint37> reduced=uint37
/// @definition.type symbol=UnsignedFixed source="type UnsignedFixed = Unsigned<uint37>" value=math.integer.Unsigned<uint37> reduced=uint37
/// @resolution.name source=Unsigned target=math.integer.Unsigned

type SignedPointer = Unsigned<isize>;
/// @type.symbol symbol=SignedPointer source="type SignedPointer = Unsigned<isize>" type=math.integer.Unsigned<isize> reduced=usize
/// @definition.type symbol=SignedPointer source="type SignedPointer = Unsigned<isize>" value=math.integer.Unsigned<isize> reduced=usize
/// @resolution.name source=Unsigned target=math.integer.Unsigned

type UnsignedPointer = Unsigned<usize>;
/// @type.symbol symbol=UnsignedPointer source="type UnsignedPointer = Unsigned<usize>" type=math.integer.Unsigned<usize> reduced=usize
/// @definition.type symbol=UnsignedPointer source="type UnsignedPointer = Unsigned<usize>" value=math.integer.Unsigned<usize> reduced=usize
/// @resolution.name source=Unsigned target=math.integer.Unsigned

/// @generic.instance id=math.integer.Unsigned<int37> template=math.integer.Unsigned arguments=(int37)
/// @generic.instance id=math.integer.Unsigned<isize> template=math.integer.Unsigned arguments=(isize)
/// @generic.instance id=math.integer.Unsigned<uint37> template=math.integer.Unsigned arguments=(uint37)
/// @generic.instance id=math.integer.Unsigned<usize> template=math.integer.Unsigned arguments=(usize)
"#,
    );
}
