use crate::tests::{DirRows, TestSession};

#[test]
fn test_newtype_over_interface_requires_explicit_implements() {
    let session = TestSession::single(
        r#"
interface Writer {
    write(bytes: readonly uint8[]): usize;
}

newtype NamedWriter = Writer;

struct Buffer {
    write(bytes: readonly uint8[]): usize {
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
    write(bytes: readonly uint8[]): usize;
}

newtype NamedWriter = Writer;

struct Buffer {
    write(bytes: readonly uint8[]): usize {
        bytes.length
    }
}

const writer: NamedWriter = Buffer {};

=== checked ===
interface Writer {
/// @type.symbol symbol=Writer type=Writer
/// @definition.interface symbol=Writer
/// @definition.method symbol=Writer.write source="write(bytes: readonly uint8[]): usize" slot=write type=(this: this, readonly Array<uint8>) => usize

    write(bytes: readonly uint8[]): usize;
    /// @type.symbol symbol=Writer.write source="write(bytes: readonly uint8[]): usize" type=(this: this, readonly Array<uint8>) => usize
    /// @type.symbol symbol=Writer.write.bytes source="bytes: readonly uint8[]" type=readonly Array<uint8>

}

newtype NamedWriter = Writer;
/// @type.symbol symbol=NamedWriter source="newtype NamedWriter = Writer" type=NamedWriter
/// @definition.newtype symbol=NamedWriter source="newtype NamedWriter = Writer" backing=Writer
/// @resolution.name source=Writer target=Writer

struct Buffer {
/// @type.symbol symbol=Buffer type=Buffer
/// @definition.struct symbol=Buffer
/// @definition.method symbol=Buffer.write slot=write type=<comptime Buffer.write.L0: memory.lifetime.Lifetime>(this: Borrowed<this, Buffer.write.L0, "exclusive">, readonly Array<uint8>) => usize

    write(bytes: readonly uint8[]): usize {
    /// @generic.template symbol=Buffer.write parameters=(comptime L0: memory.lifetime.Lifetime)
    /// @type.symbol symbol=Buffer.write type=<comptime Buffer.write.L0: memory.lifetime.Lifetime>(this: Borrowed<this, Buffer.write.L0, "exclusive">, readonly Array<uint8>) => usize
    /// @type.symbol symbol=Buffer.write.bytes source="bytes: readonly uint8[]" type=readonly Array<uint8>

        bytes.length
        /// @resolution.name source=bytes target=Buffer.write.bytes
        /// @resolution.member source=bytes.length receiver=readonly Array<uint8> kind=symbol target=collections.array.length#2

    }
}

const writer: NamedWriter = Buffer {};
/// @type.symbol symbol=writer source=writer type=NamedWriter
/// @resolution.pattern source=writer kind=binding target=writer
/// @resolution.name source=NamedWriter target=NamedWriter
/// @resolution.name source=Buffer target=Buffer
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'Buffer' is not assignable to type 'NamedWriter'"
/// @diagnostic.label line=14 column=29 span="Buffer {}" line_source="const writer: NamedWriter = Buffer {};"
/// @diagnostic.related line=14 column=15 span="NamedWriter" line_source="const writer: NamedWriter = Buffer {};" message="expected due to this annotation"
"#,
    );
}
