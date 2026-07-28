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
/// @definition.extends symbol=Document source=Base target=Base
/// @resolution.name source=Base target=Base
"#,
    );
}

#[test]
fn test_class_extends_clause_requires_class_base() {
    let session = TestSession::single(
        r#"
interface Drawable {}

class Document extends Drawable {}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Drawable {}

class Document extends Drawable {}

=== checked ===
interface Drawable {}
/// @type.symbol symbol=Drawable source="interface Drawable {}" type=Drawable
/// @definition.interface symbol=Drawable source="interface Drawable {}"

class Document extends Drawable {}
/// @type.symbol symbol=Document source="class Document extends Drawable {}" type=Document
/// @definition.class symbol=Document source="class Document extends Drawable {}"
/// @resolution.name source=Drawable target=Drawable
"#,
        r#"
/// @diagnostic.error id=does-not-extend message="type 'Document' does not extend 'Drawable'"
/// @diagnostic.label line=4 column=24 span="Drawable" line_source="class Document extends Drawable {}"
"#,
    );
}

#[test]
fn test_class_extends_rejects_type_alias_base() {
    let session = TestSession::single(
        r#"
class Base {}
type Alias = Base;

class Document extends Alias {}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Base {}
type Alias = Base;

class Document extends Alias {}

=== checked ===
class Base {}
/// @type.symbol symbol=Base source="class Base {}" type=Base
/// @definition.class symbol=Base source="class Base {}"

type Alias = Base;
/// @type.symbol symbol=Alias source="type Alias = Base" type=Base
/// @definition.type symbol=Alias source="type Alias = Base" value=Base
/// @resolution.name source=Base target=Base

class Document extends Alias {}
/// @type.symbol symbol=Document source="class Document extends Alias {}" type=Document
/// @definition.class symbol=Document source="class Document extends Alias {}"
/// @resolution.name source=Alias target=Alias
"#,
        r#"
/// @diagnostic.error id=does-not-extend message="type 'Document' does not extend 'Alias'"
/// @diagnostic.label line=5 column=24 span="Alias" line_source="class Document extends Alias {}"
"#,
    );
}

#[test]
fn test_class_extends_rejects_union_base() {
    let session = TestSession::single(
        r#"
class Base {}
class Other {}

class Document extends Base | Other {}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Base {}
class Other {}

class Document extends Base | Other {}

=== checked ===
class Base {}
/// @type.symbol symbol=Base source="class Base {}" type=Base
/// @definition.class symbol=Base source="class Base {}"

class Other {}
/// @type.symbol symbol=Other source="class Other {}" type=Other
/// @definition.class symbol=Other source="class Other {}"

class Document extends Base | Other {}
/// @type.symbol symbol=Document source="class Document extends Base | Other {}" type=Document
/// @definition.class symbol=Document source="class Document extends Base | Other {}"
/// @resolution.name source=Base target=Base
/// @resolution.name source=Other target=Other
"#,
        r#"
/// @diagnostic.error id=does-not-extend message="type 'Document' does not extend 'Base | Other'"
/// @diagnostic.label line=5 column=29 span="|" line_source="class Document extends Base | Other {}"
"#,
    );
}

#[test]
fn test_class_implements_interface_members() {
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
/// @definition.method symbol=Printable.print source="print(): void" slot=print type=(this: this) => void

    print(): void;
    /// @type.symbol symbol=Printable.print source="print(): void" type=(this: this) => void

}

class Document implements Printable {
/// @type.symbol symbol=Document type=Document
/// @definition.class symbol=Document
/// @definition.where symbol=Document source=Printable relation=satisfies left=this right=Printable
/// @definition.implements symbol=Document source=Printable target=Printable
/// @definition.method symbol=Document.print source="print(): void {}" slot=print type=(this: this) => void
/// @resolution.name source=Printable target=Printable

    print(): void {}
    /// @type.symbol symbol=Document.print source="print(): void {}" type=(this: this) => void

}
"#,
    );
}

#[test]
fn test_class_implements_rejects_type_alias_interface() {
    let session = TestSession::single(
        r#"
interface Printable {
    print(): void;
}
type Alias = Printable;

class Document implements Alias {
    print(): void {}
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Printable {
    print(): void;
}
type Alias = Printable;

class Document implements Alias {
    print(): void {}
}

=== checked ===
interface Printable {
/// @type.symbol symbol=Printable type=Printable
/// @definition.interface symbol=Printable
/// @definition.method symbol=Printable.print source="print(): void" slot=print type=(this: this) => void

    print(): void;
    /// @type.symbol symbol=Printable.print source="print(): void" type=(this: this) => void

}
type Alias = Printable;
/// @type.symbol symbol=Alias source="type Alias = Printable" type=Printable
/// @definition.type symbol=Alias source="type Alias = Printable" value=Printable
/// @resolution.name source=Printable target=Printable

class Document implements Alias {
/// @type.symbol symbol=Document type=Document
/// @definition.class symbol=Document
/// @definition.method symbol=Document.print source="print(): void {}" slot=print type=(this: this) => void
/// @resolution.name source=Alias target=Alias

    print(): void {}
    /// @type.symbol symbol=Document.print source="print(): void {}" type=(this: this) => void

}
"#,
        r#"
/// @diagnostic.error id=implementation-target-not-interface message="type 'Document' can only implement interfaces, not 'Alias'"
/// @diagnostic.label line=7 column=27 span="Alias" line_source="class Document implements Alias {"
"#,
    );
}

#[test]
fn test_class_implements_clause_requires_members() {
    let session = TestSession::single(
        r#"
interface Drawable {
    draw(): void;
}

class Point implements Drawable {
    x: int32 = 0;
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Drawable {
    draw(): void;
}

class Point implements Drawable {
    x: int32 = 0;
}

=== checked ===
interface Drawable {
/// @type.symbol symbol=Drawable type=Drawable
/// @definition.interface symbol=Drawable
/// @definition.method symbol=Drawable.draw source="draw(): void" slot=draw type=(this: this) => void

    draw(): void;
    /// @type.symbol symbol=Drawable.draw source="draw(): void" type=(this: this) => void

}

class Point implements Drawable {
/// @type.symbol symbol=Point type=Point
/// @definition.class symbol=Point
/// @definition.where symbol=Point source=Drawable relation=satisfies left=this right=Drawable
/// @definition.implements symbol=Point source=Drawable target=Drawable
/// @definition.field symbol=Point.x source="x: int32 = 0" key=x type=int32
/// @resolution.name source=Drawable target=Drawable

    x: int32 = 0;
    /// @type.symbol symbol=Point.x source="x: int32 = 0" type=int32

}
"#,
        r#"
/// @diagnostic.error id=interface-not-implemented message="type 'Point' does not implement interface 'Drawable'"
/// @diagnostic.label line=6 column=24 span="Drawable" line_source="class Point implements Drawable {"
"#,
    );
}

#[test]
fn test_class_implements_clause_requires_inherited_members() {
    let session = TestSession::single(
        r#"
interface Named {
    name(): string;
}

interface Drawable extends Named {
    draw(): void;
}

class Point implements Drawable {
    draw(): void {}
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Named {
    name(): string;
}

interface Drawable extends Named {
    draw(): void;
}

class Point implements Drawable {
    draw(): void {}
}

=== checked ===
interface Named {
/// @type.symbol symbol=Named type=Named
/// @definition.interface symbol=Named
/// @definition.method symbol=Named.name source="name(): string" slot=name type=(this: this) => string

    name(): string;
    /// @type.symbol symbol=Named.name source="name(): string" type=(this: this) => string

}

interface Drawable extends Named {
/// @type.symbol symbol=Drawable type=Drawable
/// @definition.interface symbol=Drawable
/// @definition.extends symbol=Drawable source=Named target=Named
/// @definition.method symbol=Drawable.draw source="draw(): void" slot=draw type=(this: this) => void
/// @resolution.name source=Named target=Named

    draw(): void;
    /// @type.symbol symbol=Drawable.draw source="draw(): void" type=(this: this) => void

}

class Point implements Drawable {
/// @type.symbol symbol=Point type=Point
/// @definition.class symbol=Point
/// @definition.where symbol=Point source=Drawable relation=satisfies left=this right=Drawable
/// @definition.implements symbol=Point source=Drawable target=Drawable
/// @definition.method symbol=Point.draw source="draw(): void {}" slot=draw type=(this: this) => void
/// @resolution.name source=Drawable target=Drawable

    draw(): void {}
    /// @type.symbol symbol=Point.draw source="draw(): void {}" type=(this: this) => void

}
"#,
        r#"
/// @diagnostic.error id=interface-not-implemented message="type 'Point' does not implement interface 'Drawable'"
/// @diagnostic.label line=10 column=24 span="Drawable" line_source="class Point implements Drawable {"
"#,
    );
}

#[test]
fn test_class_extends_rejects_circular_heritage() {
    let session = TestSession::single(
        r#"
class Left extends Right {}
class Right extends Left {}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Left extends Right {}
class Right extends Left {}

=== checked ===
class Left extends Right {}
/// @type.symbol symbol=Left source="class Left extends Right {}" type=Left
/// @definition.class symbol=Left source="class Left extends Right {}"
/// @definition.extends symbol=Left source=Right target=Right
/// @resolution.name source=Right target=Right

class Right extends Left {}
/// @type.symbol symbol=Right source="class Right extends Left {}" type=Right
/// @definition.class symbol=Right source="class Right extends Left {}"
/// @definition.extends symbol=Right source=Left target=Left
/// @resolution.name source=Left target=Left
"#,
        r#"
/// @diagnostic.error id=circular-heritage message="type 'Right' has circular heritage"
/// @diagnostic.label line=3 column=21 span="Left" line_source="class Right extends Left {}"
/// @diagnostic.error id=circular-heritage message="type 'Left' has circular heritage"
/// @diagnostic.label line=2 column=20 span="Right" line_source="class Left extends Right {}"
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
/// @definition.method symbol=Document.print source="override print(): void {}" slot=print override=true type=(this: this) => void

    override print(): void {}
    /// @type.symbol symbol=Document.print source="override print(): void {}" type=(this: this) => void

}
"#,
        r#"
/// @diagnostic.error id=invalid-override message="'print' does not override an inherited member"
/// @diagnostic.label line=3 column=14 span="print" line_source="override print(): void {}"
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
/// @definition.method symbol=Base.print source="virtual print(): void {}" slot=print abstraction=virtual type=(this: this) => void

    virtual print(): void {}
    /// @type.symbol symbol=Base.print source="virtual print(): void {}" type=(this: this) => void

}

class Document extends Base {
/// @type.symbol symbol=Document type=Document
/// @definition.class symbol=Document
/// @definition.extends symbol=Document source=Base target=Base
/// @definition.method symbol=Document.print source="print(): void {}" slot=print type=(this: this) => void
/// @resolution.name source=Base target=Base

    print(): void {}
    /// @type.symbol symbol=Document.print source="print(): void {}" type=(this: this) => void

}
"#,
        r#"
/// @diagnostic.error id=missing-override message="'print' shadows an inherited member and must be declared 'override'"
/// @diagnostic.label line=7 column=5 span="print" line_source="print(): void {}"
/// @diagnostic.help message="add the 'override' modifier"
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
/// @definition.method symbol=Base.print source="print(): void {}" slot=print type=(this: this) => void

    print(): void {}
    /// @type.symbol symbol=Base.print source="print(): void {}" type=(this: this) => void

}

class Document extends Base {
/// @type.symbol symbol=Document type=Document
/// @definition.class symbol=Document
/// @definition.extends symbol=Document source=Base target=Base
/// @definition.method symbol=Document.print source="override print(): void {}" slot=print override=true type=(this: this) => void
/// @resolution.name source=Base target=Base

    override print(): void {}
    /// @type.symbol symbol=Document.print source="override print(): void {}" type=(this: this) => void

}
"#,
        r#"
/// @diagnostic.error id=override-not-virtual message="cannot override 'print': the inherited member is not virtual"
/// @diagnostic.label line=7 column=14 span="print" line_source="override print(): void {}"
/// @diagnostic.help message="declare the inherited member 'virtual' or 'abstract'"
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
/// @definition.class symbol=Packet source="final class Packet {}" final=true

class Header extends Packet {}
/// @type.symbol symbol=Header source="class Header extends Packet {}" type=Header
/// @definition.class symbol=Header source="class Header extends Packet {}"
/// @definition.extends symbol=Header source=Packet target=Packet
/// @resolution.name source=Packet target=Packet
"#,
        r#"
/// @diagnostic.error id=final-class-extended message="final class 'Packet' cannot be extended"
/// @diagnostic.label line=4 column=7 span="Header" line_source="class Header extends Packet {}"
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
/// @definition.method symbol=Writer.write source="abstract write(value: string): void" slot=write abstraction=abstract type=(this: this, string) => void

    abstract write(value: string): void;
    /// @type.symbol symbol=Writer.write source="abstract write(value: string): void" type=(this: this, string) => void
    /// @type.symbol symbol=Writer.write.value source="value: string" type=string

}
"#,
        r#"
/// @diagnostic.error id=abstract-member-in-concrete-class message="abstract member 'write' requires an abstract class"
/// @diagnostic.label line=3 column=14 span="write" line_source="abstract write(value: string): void;"
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
/// @definition.class symbol=Writer abstract=true
/// @definition.method symbol=Writer.write source="abstract write(value: string): void" slot=write abstraction=abstract type=(this: this, string) => void

    abstract write(value: string): void;
    /// @type.symbol symbol=Writer.write source="abstract write(value: string): void" type=(this: this, string) => void
    /// @type.symbol symbol=Writer.write.value source="value: string" type=string

}

class FileWriter extends Writer {}
/// @type.symbol symbol=FileWriter source="class FileWriter extends Writer {}" type=FileWriter
/// @definition.class symbol=FileWriter source="class FileWriter extends Writer {}"
/// @definition.extends symbol=FileWriter source=Writer target=Writer
/// @resolution.name source=Writer target=Writer
"#,
        r#"
/// @diagnostic.error id=unimplemented-abstract-member message="abstract member 'write' is not implemented"
/// @diagnostic.label line=6 column=7 span="FileWriter" line_source="class FileWriter extends Writer {}"
/// @diagnostic.help message="implement the member or declare the class 'abstract'"
"#,
    );
}

#[test]
fn test_structural_records_never_assign_into_nominal_classes() {
    let session = TestSession::single(
        r#"
declare const record: Record<string, int32>;
const map: Map<string, int32> = record;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const record: Record<string, int32>;
const map: Map<string, int32> = record;

=== checked ===
declare const record: Record<string, int32>;
/// @type.symbol symbol=record source=record type=Record<string, int32> reduced={ [P: string]: int32 }
/// @resolution.pattern source=record kind=binding target=record
/// @resolution.name source=Record target=types.object.Record

const map: Map<string, int32> = record;
/// @type.symbol symbol=map source=map type=Map<string, int32>
/// @resolution.pattern source=map kind=binding target=map
/// @resolution.name source=Map target=collections.map.Map
/// @resolution.name source=record target=record
/// @resolution.place source=record placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=record root=record

/// @generic.instance id="Map<string, int32>" template=collections.map.Map arguments=(string, int32)
/// @generic.instance id="Record<string, int32>" template=types.object.Record arguments=(string, int32)
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'Record<string, int32>' is not assignable to type 'Map<string, int32>'"
/// @diagnostic.label line=3 column=33 span="record" line_source="const map: Map<string, int32> = record;"
/// @diagnostic.related line=3 column=12 span="Map" line_source="const map: Map<string, int32> = record;" message="expected due to this annotation"
/// @diagnostic.note message="'Record<string, int32>' reduces to '{ [P: string]: int32 }'"
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
/// @definition.class symbol=Writer source="abstract class Writer {}" abstract=true

new Writer();
/// @resolution.name source=Writer target=Writer
"#,
        r#"
/// @diagnostic.error id=cannot-construct-abstract-type message="abstract class 'Writer' cannot be constructed"
/// @diagnostic.label line=4 column=1 span="new Writer()" line_source="new Writer();"
/// @diagnostic.help message="construct a concrete subclass instead"
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
/// @definition.method symbol=Base.parse slot=parse abstraction=virtual type=(this: this, string) => string

    virtual parse(value: string): string {
    /// @type.symbol symbol=Base.parse type=(this: this, string) => string
    /// @type.symbol symbol=Base.parse.value source="value: string" type=string

        return value;
        /// @resolution.name source=value target=Base.parse.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=Base.parse.value

    }
}

class Parser extends Base {
/// @type.symbol symbol=Parser type=Parser
/// @definition.class symbol=Parser
/// @definition.extends symbol=Parser source=Base target=Base
/// @definition.method symbol=Parser.parse slot=parse override=true type=(this: this, string) => int32
/// @resolution.name source=Base target=Base

    override parse(value: string): int32 {
    /// @type.symbol symbol=Parser.parse type=(this: this, string) => int32
    /// @type.symbol symbol=Parser.parse.value source="value: string" type=string

        return 1;
    }
}
"#,
        r#"
/// @diagnostic.error id=incompatible-override message="override 'parse' has type '(string) => int32', which is not assignable to the inherited type '(string) => string'"
/// @diagnostic.label line=9 column=14 span="parse" line_source="override parse(value: string): int32 {"
"#,
    );
}
