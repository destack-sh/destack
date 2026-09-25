use crate::tests::{DirRows, TestSession};

/// A newtype over an interface accepts only types that implement it by name.
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

    session.assert_dir_and_diagnostics(
        "main.tspp",
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

=== dir ===
interface Writer {
/// @generic.template symbol=Writer parameters=(this: Writer)
/// @type.symbol symbol=Writer type=Writer
/// @definition.interface symbol=Writer template=(this: Writer)
/// @definition.where symbol=Writer relation=satisfies left=this right=Writer
/// @definition.method symbol=Writer.write source="write(bytes: readonly uint8[]): usize" slot=write type=(readonly uint8[]) => usize

    write(bytes: readonly uint8[]): usize;
    /// @type.symbol symbol=Writer.write source="write(bytes: readonly uint8[]): usize" type=(readonly uint8[]) => usize
    /// @type.symbol symbol=Writer.write.bytes source="bytes: readonly uint8[]" type=readonly uint8[]

}

newtype NamedWriter = Writer;
/// @type.symbol symbol=NamedWriter source="newtype NamedWriter = Writer" type=NamedWriter
/// @definition.newtype symbol=NamedWriter source="newtype NamedWriter = Writer" backing=Writer constructors=[(Writer) => NamedWriter]
/// @resolution.name source=Writer target=Writer

struct Buffer {
/// @type.symbol symbol=Buffer type=Buffer
/// @definition.struct symbol=Buffer
/// @definition.method symbol=Buffer.write slot=write type=<Buffer.write.'a>(this: &Buffer.write.'a readonly Buffer, readonly uint8[]) => usize

    write(bytes: readonly uint8[]): usize {
    /// @generic.template symbol=Buffer.write parameters=('a)
    /// @type.symbol symbol=Buffer.write type=<Buffer.write.'a>(this: &Buffer.write.'a readonly Buffer, readonly uint8[]) => usize
    /// @type.symbol symbol=Buffer.write.this type=&Buffer.write.'a readonly Buffer
    /// @type.symbol symbol=Buffer.write.bytes source="bytes: readonly uint8[]" type=readonly uint8[]

        bytes.length
        /// @resolution.name source=bytes target=Buffer.write.bytes
        /// @resolution.member source=bytes.length receiver=readonly uint8[] type=isize kind=call target="length(parameters=(), arguments=(), return=isize, regions=(\"managed\" & \"local\"))"
        /// @resolution.place source=bytes placement="local" lifetime="frame" access="readonly"
        /// @resolution.access source=bytes root=Buffer.write.bytes
        /// @generic.instantiation id="length<uint8, \"managed\" & \"local\">" template=length arguments=(uint8, "managed" & "local")

    }
}

const writer: NamedWriter = Buffer {};
/// @type.symbol symbol=writer source=writer type=NamedWriter
/// @resolution.pattern source=writer kind=binding target=writer
/// @resolution.name source=NamedWriter target=NamedWriter
/// @resolution.name source=Buffer target=Buffer
"#,
        r#"
/// @diagnostic.error id=return-not-assignable message="type 'isize' is not assignable to the declared result type 'usize'"
/// @diagnostic.label line=9 column=43 span="{\n        bytes.length\n    }" line_source="write(bytes: readonly uint8[]): usize {"
/// @diagnostic.error id=not-assignable message="type 'Buffer' is not assignable to type 'NamedWriter'"
/// @diagnostic.label line=14 column=29 span="Buffer {}" line_source="const writer: NamedWriter = Buffer {};"
/// @diagnostic.related line=14 column=15 span="NamedWriter" line_source="const writer: NamedWriter = Buffer {};" message="expected due to this annotation"
"#,
    );
}

/// A newtype constructor accepts a shifted literal argument.
#[test]
fn test_construct_newtype_constant_from_shifted_literal() {
    // a newtype constructor call over a const shift adopts its annotation
    let session = TestSession::single(
        r#"
export newtype Mask = uint32;

export const CREATE: Mask = Mask(1 << 0);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
export newtype Mask = uint32;

export const CREATE: Mask = Mask(1 << 0);

=== dir ===
export newtype Mask = uint32;
/// @type.symbol symbol=Mask source="export newtype Mask = uint32" type=Mask
/// @definition.newtype symbol=Mask source="export newtype Mask = uint32" backing=uint32 constructors=[(uint32) => Mask]

export const CREATE: Mask = Mask(1 << 0);
/// @type.symbol symbol=CREATE source=CREATE type=Mask
/// @resolution.pattern source=CREATE kind=binding target=CREATE
/// @resolution.name source=Mask target=Mask
/// @resolution.name source=Mask target=Mask
/// @resolution.construct source="Mask(1 << 0)" parameters=(uint32) arguments=(provided(1 << 0) as uint32) return=Mask kind=newtype target=Mask backing=uint32
/// @resolution.operator source="1 << 0" type=1 operator="<<" kind=builtin operands=[1 as 1 families=(integer), 0 as 0 families=(integer)]
"#,
        r#"

"#,
    );
}
