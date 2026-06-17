use crate::tests::{DirRows, TestSession};

#[test]
fn test_newtype_over_interface_requires_explicit_implements() {
    let session = TestSession::single(
        r#"
interface Writer {
    write(bytes: readonly uint8[]): uint;
}

newtype NamedWriter = Writer;

struct Buffer {
    write(bytes: readonly uint8[]): uint {
        bytes.length
    }
}

const writer: NamedWriter = Buffer {};
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Writer {
    write(bytes: readonly uint8[]): uint;
}

newtype NamedWriter = Writer;

struct Buffer {
    write(bytes: readonly uint8[]): uint {
        bytes.length
    }
}

const writer: NamedWriter = Buffer {};

=== checked ===
interface Writer {
/// @type.symbol symbol=Writer type=Writer
/// @definition.interface symbol=Writer
/// @definition.method symbol=Writer.write source="write(bytes: readonly uint8[]): uint" key=write type=(readonly uint8[]) => uint

    write(bytes: readonly uint8[]): uint;
    /// @type.symbol symbol=Writer.write source="write(bytes: readonly uint8[]): uint" type=(readonly uint8[]) => uint
    /// @type.symbol symbol=bytes type=readonly uint8[]

}

newtype NamedWriter = Writer;
/// @type.symbol symbol=NamedWriter source="newtype NamedWriter = Writer" type=NamedWriter
/// @definition.newtype symbol=NamedWriter source="newtype NamedWriter = Writer" value=Writer
/// @resolution.name source=Writer target=Writer

struct Buffer {
/// @type.symbol symbol=Buffer type=Buffer
/// @definition.struct symbol=Buffer
/// @definition.method symbol=Buffer.write slot=write type=(this: Buffer, readonly uint8[]) => uint

    write(bytes: readonly uint8[]): uint {
    /// @type.symbol symbol=Buffer.write type=(this: Buffer, readonly uint8[]) => uint
    /// @type.symbol symbol=bytes type=readonly uint8[]

        bytes.length
        /// @type.node source=bytes.length type=usize
        /// @resolution.name source=bytes target=bytes
        /// @resolution.member source=bytes.length receiver=readonly uint8[] kind=builtin builtin=length

    }
}

const writer: NamedWriter = Buffer {};
/// @type.symbol symbol=writer source=writer type=NamedWriter
/// @resolution.name source=NamedWriter target=NamedWriter
/// @resolution.name source=Buffer target=Buffer
"#,
        r#"
/// @diagnostic.error code=EC200 message="type 'Buffer' is not assignable to type 'NamedWriter'"
/// @diagnostic.label line=14 column=29 source="const writer: NamedWriter = Buffer {};"
"#,
    );
}
