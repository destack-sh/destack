use crate::tests::{DirRows, TestSession};

#[test]
fn test_declared_type_relations_are_preserved() {
    let session = TestSession::single(
        r#"
class Base {}

interface Printable {
    print(): void;
}

class Document extends Base implements Printable {
    print(): void {}
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Base {}

interface Printable {
    print(): void;
}

class Document extends Base implements Printable {
    print(): void {}
}

=== checked ===
class Base {}
/// @type.symbol symbol=Base source="class Base {}" type=Base
/// @definition.class symbol=Base source="class Base {}"

interface Printable {
/// @type.symbol symbol=Printable type=Printable
/// @definition.interface symbol=Printable
/// @definition.method symbol=Printable.print source="print(): void" slot=print type=(this: Printable) => void

    print(): void;
    /// @type.symbol symbol=Printable.print source="print(): void" type=(this: Printable) => void

}

class Document extends Base implements Printable {
/// @type.symbol symbol=Document type=Document
/// @definition.class symbol=Document
/// @definition.method symbol=Document.print source="print(): void {}" slot=print type=(this: Document) => void
/// @resolution.name source=Base target=Base
/// @resolution.name source=Printable target=Printable

    print(): void {}
    /// @type.symbol symbol=Document.print source="print(): void {}" type=(this: Document) => void

}
"#,
    );
}
