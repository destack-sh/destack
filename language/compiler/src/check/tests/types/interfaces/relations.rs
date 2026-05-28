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
class Base {}
/// @type.symbol symbol=Base type=Base

interface Printable {
/// @type.symbol symbol=Printable type=Printable

    print(): void;
    /// @type.symbol symbol=Printable.print type=(this: Printable) => void
}

class Document extends Base implements Printable {
/// @type.symbol symbol=Document type=Document
/// @relation.entry symbol=Document kind=extends type=Base
/// @relation.entry symbol=Document kind=implements type=Printable

    print(): void {}
    /// @type.symbol symbol=Document.print type=(this: Document) => void
}
"#,
    );
}
