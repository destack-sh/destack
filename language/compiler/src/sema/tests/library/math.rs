use crate::tests::{DirRows, TestSession};

#[test]
fn test_unsigned_preserves_integer_width() {
    let session = TestSession::single(
        r#"
import { Unsigned } from "tspp:math";

type SignedFixed = Unsigned<int37>;
type UnsignedFixed = Unsigned<uint37>;
type SignedPointer = Unsigned<isize>;
type UnsignedPointer = Unsigned<usize>;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { Unsigned } from "tspp:math";

type SignedFixed = Unsigned<int37>;
type UnsignedFixed = Unsigned<uint37>;
type SignedPointer = Unsigned<isize>;
type UnsignedPointer = Unsigned<usize>;

=== dir ===
import { Unsigned } from "tspp:math";

type SignedFixed = Unsigned<int37>;
/// @type.symbol symbol=SignedFixed source="type SignedFixed = Unsigned<int37>" type=uint37
/// @generic.instance id=Unsigned<int37> template=Unsigned arguments=(int37)
/// @definition.type symbol=SignedFixed source="type SignedFixed = Unsigned<int37>" value=Unsigned<int37>
/// @resolution.name source=Unsigned target=Unsigned

type UnsignedFixed = Unsigned<uint37>;
/// @type.symbol symbol=UnsignedFixed source="type UnsignedFixed = Unsigned<uint37>" type=uint37
/// @generic.instance id=Unsigned<uint37> template=Unsigned arguments=(uint37)
/// @definition.type symbol=UnsignedFixed source="type UnsignedFixed = Unsigned<uint37>" value=Unsigned<uint37>
/// @resolution.name source=Unsigned target=Unsigned

type SignedPointer = Unsigned<isize>;
/// @type.symbol symbol=SignedPointer source="type SignedPointer = Unsigned<isize>" type=usize
/// @generic.instance id=Unsigned<isize> template=Unsigned arguments=(isize)
/// @definition.type symbol=SignedPointer source="type SignedPointer = Unsigned<isize>" value=Unsigned<isize>
/// @resolution.name source=Unsigned target=Unsigned

type UnsignedPointer = Unsigned<usize>;
/// @type.symbol symbol=UnsignedPointer source="type UnsignedPointer = Unsigned<usize>" type=usize
/// @generic.instance id=Unsigned<usize> template=Unsigned arguments=(usize)
/// @definition.type symbol=UnsignedPointer source="type UnsignedPointer = Unsigned<usize>" value=Unsigned<usize>
/// @resolution.name source=Unsigned target=Unsigned
"#,
    );
}
