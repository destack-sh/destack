use crate::tests::{DirRows, TestSession};

#[test]
fn test_class_extends_base_class() {
    let session = TestSession::single(
        r#"
class Base {}

class Document extends Base {}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Base {}

class Document extends Base {}

=== checked ===
class Base {}
/// @type.symbol symbol=Base source="class Base {}" type=Base
/// @definition.class symbol=Base source="class Base {}"

class Document extends Base {}
/// @type.symbol symbol=Document source="class Document extends Base {}" type=Document
/// @definition.class symbol=Document source="class Document extends Base {}"
/// @resolution.name source=Base target=Base
"#,
    );
}

#[test]
fn test_class_implements_interface_contract() {
    let session = TestSession::single(
        r#"
interface Printable {
    print(): void;
}

class Document implements Printable {
    print(): void {}
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Printable {
    print(): void;
}

class Document implements Printable {
    print(): void {}
}

=== checked ===
interface Printable {
/// @type.symbol symbol=Printable type=Printable
/// @definition.interface symbol=Printable
/// @definition.method symbol=Printable.print source="print(): void" slot=print type=(this: Printable) => void

    print(): void;
    /// @type.symbol symbol=Printable.print source="print(): void" type=(this: Printable) => void

}

class Document implements Printable {
/// @type.symbol symbol=Document type=Document
/// @definition.class symbol=Document
/// @definition.method symbol=Document.print source="print(): void {}" slot=print type=(this: Document) => void
/// @resolution.name source=Printable target=Printable

    print(): void {}
    /// @type.symbol symbol=Document.print source="print(): void {}" type=(this: Document) => void

}
"#,
    );
}

#[test]
fn test_override_without_inherited_member_reports_error() {
    let session = TestSession::single(
        r#"
class Document {
    override print(): void {}
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Document {
    override print(): void {}
}

=== checked ===
class Document {
/// @type.symbol symbol=Document type=Document
/// @definition.class symbol=Document
/// @definition.method symbol=Document.print source="override print(): void {}" slot=print type=(this: Document) => void

    override print(): void {}
    /// @type.symbol symbol=Document.print source="override print(): void {}" type=(this: Document) => void
}
"#,
        r#"
/// @diagnostic.error code=EC600 message="'print' does not override an inherited member"
/// @diagnostic.label line=3 column=5 source="override print(): void {}"
"#,
    );
}

#[test]
fn test_shadowing_inherited_member_requires_override() {
    let session = TestSession::single(
        r#"
class Base {
    virtual print(): void {}
}

class Document extends Base {
    print(): void {}
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Base {
    virtual print(): void {}
}

class Document extends Base {
    print(): void {}
}

=== checked ===
class Base {
/// @type.symbol symbol=Base type=Base
/// @definition.class symbol=Base
/// @definition.method symbol=Base.print source="virtual print(): void {}" slot=print type=(this: Base) => void

    virtual print(): void {}
    /// @type.symbol symbol=Base.print source="virtual print(): void {}" type=(this: Base) => void
}

class Document extends Base {
/// @type.symbol symbol=Document type=Document
/// @definition.class symbol=Document
/// @definition.method symbol=Document.print source="print(): void {}" slot=print type=(this: Document) => void
/// @resolution.name source=Base target=Base

    print(): void {}
    /// @type.symbol symbol=Document.print source="print(): void {}" type=(this: Document) => void
}
"#,
        r#"
/// @diagnostic.error code=EC606 message="'print' shadows an inherited member and must be declared 'override'"
/// @diagnostic.label line=7 column=5 source="print(): void {}"
"#,
    );
}

#[test]
fn test_override_requires_virtual_inherited_member() {
    let session = TestSession::single(
        r#"
class Base {
    print(): void {}
}

class Document extends Base {
    override print(): void {}
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Base {
    print(): void {}
}

class Document extends Base {
    override print(): void {}
}

=== checked ===
class Base {
/// @type.symbol symbol=Base type=Base
/// @definition.class symbol=Base
/// @definition.method symbol=Base.print source="print(): void {}" slot=print type=(this: Base) => void

    print(): void {}
    /// @type.symbol symbol=Base.print source="print(): void {}" type=(this: Base) => void
}

class Document extends Base {
/// @type.symbol symbol=Document type=Document
/// @definition.class symbol=Document
/// @definition.method symbol=Document.print source="override print(): void {}" slot=print type=(this: Document) => void
/// @resolution.name source=Base target=Base

    override print(): void {}
    /// @type.symbol symbol=Document.print source="override print(): void {}" type=(this: Document) => void
}
"#,
        r#"
/// @diagnostic.error code=EC607 message="cannot override 'print': the inherited member is not virtual"
/// @diagnostic.label line=7 column=5 source="override print(): void {}"
"#,
    );
}

#[test]
fn test_final_class_cannot_be_extended() {
    let session = TestSession::single(
        r#"
final class Packet {}

class Header extends Packet {}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
final class Packet {}

class Header extends Packet {}

=== checked ===
final class Packet {}
/// @type.symbol symbol=Packet source="final class Packet {}" type=Packet
/// @definition.class symbol=Packet source="final class Packet {}"

class Header extends Packet {}
/// @type.symbol symbol=Header source="class Header extends Packet {}" type=Header
/// @definition.class symbol=Header source="class Header extends Packet {}"
/// @resolution.name source=Packet target=Packet
"#,
        r#"
/// @diagnostic.error code=EC608 message="final class 'Packet' cannot be extended"
/// @diagnostic.label line=4 column=1 source="class Header extends Packet {}"
"#,
    );
}

#[test]
fn test_abstract_member_requires_abstract_class() {
    let session = TestSession::single(
        r#"
class Writer {
    abstract write(value: string): void;
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Writer {
    abstract write(value: string): void;
}

=== checked ===
class Writer {
/// @type.symbol symbol=Writer type=Writer
/// @definition.class symbol=Writer
/// @definition.method symbol=Writer.write source="abstract write(value: string): void" slot=write type=(this: Writer, string) => void

    abstract write(value: string): void;
    /// @type.symbol symbol=Writer.write source="abstract write(value: string): void" type=(this: Writer, string) => void
    /// @type.symbol symbol=value source="value: string" type=string
}
"#,
        r#"
/// @diagnostic.error code=EC609 message="abstract member 'write' requires an abstract class"
/// @diagnostic.label line=3 column=5 source="abstract write(value: string): void;"
"#,
    );
}

#[test]
fn test_concrete_subclass_must_implement_abstract_member() {
    let session = TestSession::single(
        r#"
abstract class Writer {
    abstract write(value: string): void;
}

class FileWriter extends Writer {}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
abstract class Writer {
    abstract write(value: string): void;
}

class FileWriter extends Writer {}

=== checked ===
abstract class Writer {
/// @type.symbol symbol=Writer type=Writer
/// @definition.class symbol=Writer
/// @definition.method symbol=Writer.write source="abstract write(value: string): void" slot=write type=(this: Writer, string) => void

    abstract write(value: string): void;
    /// @type.symbol symbol=Writer.write source="abstract write(value: string): void" type=(this: Writer, string) => void
    /// @type.symbol symbol=value source="value: string" type=string
}

class FileWriter extends Writer {}
/// @type.symbol symbol=FileWriter source="class FileWriter extends Writer {}" type=FileWriter
/// @definition.class symbol=FileWriter source="class FileWriter extends Writer {}"
/// @resolution.name source=Writer target=Writer
"#,
        r#"
/// @diagnostic.error code=EC601 message="abstract member 'write' is not implemented"
/// @diagnostic.label line=6 column=1 source="class FileWriter extends Writer {}"
"#,
    );
}

#[test]
fn test_abstract_class_cannot_be_constructed() {
    let session = TestSession::single(
        r#"
abstract class Writer {}

new Writer();
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
abstract class Writer {}

new Writer();

=== checked ===
abstract class Writer {}
/// @type.symbol symbol=Writer source="abstract class Writer {}" type=Writer
/// @definition.class symbol=Writer source="abstract class Writer {}"

new Writer();
/// @type.node source="new Writer()" type=<error>
/// @resolution.name source=Writer target=Writer
"#,
        r#"
/// @diagnostic.error code=EC602 message="abstract class 'Writer' cannot be constructed"
/// @diagnostic.label line=4 column=1 source="new Writer();"
"#,
    );
}

#[test]
fn test_override_must_be_assignable_to_inherited_member() {
    let session = TestSession::single(
        r#"
class Base {
    virtual parse(value: string): string {
        return value;
    }
}

class Parser extends Base {
    override parse(value: string): int32 {
        return 1;
    }
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Base {
    virtual parse(value: string): string {
        return value;
    }
}

class Parser extends Base {
    override parse(value: string): int32 {
        return 1;
    }
}

=== checked ===
class Base {
/// @type.symbol symbol=Base type=Base
/// @definition.class symbol=Base
/// @definition.method symbol=Base.parse source="virtual parse(value: string): string {\n        return value;\n    }" slot=parse type=(this: Base, string) => string

    virtual parse(value: string): string {
    /// @type.symbol symbol=Base.parse type=(this: Base, string) => string
    /// @type.symbol symbol=value#1 source="value: string" type=string

        return value;
        /// @resolution.name source=value target=value#1
    }
}

class Parser extends Base {
/// @type.symbol symbol=Parser type=Parser
/// @definition.class symbol=Parser
/// @definition.method symbol=Parser.parse source="override parse(value: string): int32 {\n        return 1;\n    }" slot=parse type=(this: Parser, string) => int32
/// @resolution.name source=Base target=Base

    override parse(value: string): int32 {
    /// @type.symbol symbol=Parser.parse type=(this: Parser, string) => int32
    /// @type.symbol symbol=value#2 source="value: string" type=string

        return 1;
        /// @type.node source=1 type=int32
    }
}
"#,
        r#"
/// @diagnostic.error code=EC610 message="override 'parse' has type '(this: Parser, string) => int32', which is not assignable to the inherited type '(this: Base, string) => string'"
/// @diagnostic.label line=9 column=5 source="override parse(value: string): int32 {"
"#,
    );
}
