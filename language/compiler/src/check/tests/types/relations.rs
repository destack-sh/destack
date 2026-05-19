use super::super::snapshot::assert_check_snapshot;

#[test]
fn test_check_records_declared_type_relations() {
    assert_check_snapshot(
        r#"
class Base {}

interface Printable {
    print(): void;
}

class Document extends Base implements Printable {
    print(): void {}
}
"#,
        r#"
class Base {}
/// @type.symbol key=Base value=Base

interface Printable {
/// @type.symbol key=Printable value=Printable

    print(): void;
    /// @type.symbol key=Printable.print value=(this: Printable) => void
}

class Document extends Base implements Printable {
/// @type.symbol key=Document value=Document
/// @relation.entry key=Document kind=extends type=Base
/// @relation.entry key=Document kind=implements type=Printable

    print(): void {}
    /// @type.symbol key=Document.print value=(this: Document) => void
}

/// @type.summary types=3 nodes=0 symbols=5
/// @generic.summary parameters=0 lists=0
/// @relation.summary extends=1 implements=1
/// @extension.summary extensions=0
/// @resolution.summary names=0 labels=0 members=0 calls=0
/// @instance.summary instances=0 nodes=0
/// @capture.summary functions=0 bindings=0 directives=0 rules=0
/// @layout.summary layouts=0 types=0
"#,
    );
}
