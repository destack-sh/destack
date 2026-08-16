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

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Unsigned } from "destack:math";

type SignedFixed = Unsigned<int37>;
type UnsignedFixed = Unsigned<uint37>;
type SignedPointer = Unsigned<isize>;
type UnsignedPointer = Unsigned<usize>;

=== dir ===
import { Unsigned } from "destack:math";

type SignedFixed = Unsigned<int37>;
/// @type.symbol symbol=SignedFixed source="type SignedFixed = Unsigned<int37>" type=uint37
/// @definition.type symbol=SignedFixed source="type SignedFixed = Unsigned<int37>" value=uint37
/// @resolution.name source=Unsigned target=math.integer.Unsigned

type UnsignedFixed = Unsigned<uint37>;
/// @type.symbol symbol=UnsignedFixed source="type UnsignedFixed = Unsigned<uint37>" type=uint37
/// @definition.type symbol=UnsignedFixed source="type UnsignedFixed = Unsigned<uint37>" value=uint37
/// @resolution.name source=Unsigned target=math.integer.Unsigned

type SignedPointer = Unsigned<isize>;
/// @type.symbol symbol=SignedPointer source="type SignedPointer = Unsigned<isize>" type=usize
/// @definition.type symbol=SignedPointer source="type SignedPointer = Unsigned<isize>" value=usize
/// @resolution.name source=Unsigned target=math.integer.Unsigned

type UnsignedPointer = Unsigned<usize>;
/// @type.symbol symbol=UnsignedPointer source="type UnsignedPointer = Unsigned<usize>" type=usize
/// @definition.type symbol=UnsignedPointer source="type UnsignedPointer = Unsigned<usize>" value=usize
/// @resolution.name source=Unsigned target=math.integer.Unsigned
"#,
    );
}
