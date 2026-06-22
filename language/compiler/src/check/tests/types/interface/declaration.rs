use crate::tests::{DirRows, TestSession};

#[test]
fn test_interface_declares_fields_and_methods() {
    let session = TestSession::single(
        r#"
interface Person {
    readonly id: string;
    name?: string;

    rename(value: string): void;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Person {
    readonly id: string;
    name?: string;

    rename(value: string): void;
}

=== checked ===
interface Person {
/// @type.symbol symbol=Person type=Person
/// @definition.field symbol=Person.id source="readonly id: string" key=id type=string
/// @definition.field symbol=Person.name source="name?: string" key=name type=string | undefined
/// @definition.interface symbol=Person
/// @definition.method symbol=Person.rename source="rename(value: string): void" slot=rename type=(this: Person, string) => void

    readonly id: string;
    /// @type.symbol symbol=Person.id source="readonly id: string" type=string

    name?: string;
    /// @type.symbol symbol=Person.name source="name?: string" type=string | undefined

    rename(value: string): void;
    /// @type.symbol symbol=Person.rename source="rename(value: string): void" type=(this: Person, string) => void
    /// @type.symbol symbol=value source="value: string" type=string

}
"#,
    );
}
