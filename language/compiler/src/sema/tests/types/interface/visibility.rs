use crate::tests::{DirRows, TestSession};

/// An interface method rejects a written visibility modifier.
#[test]
fn test_interface_method_rejects_a_visibility_modifier() {
    let session = TestSession::single(
        r#"
interface Reader {
    private read(): string;
}
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked().with_definitions(), r#"
=== annotated ===
interface Reader {
    private read(): string;
}

=== dir ===
interface Reader {
/// @generic.template symbol=Reader parameters=(this: Reader)
/// @type.symbol symbol=Reader type=Reader
/// @definition.interface symbol=Reader template=(this: Reader)
/// @definition.where symbol=Reader relation=satisfies left=this right=Reader
/// @definition.method symbol=Reader.read source="private read(): string" slot=read type=() => string

    private read(): string;
    /// @type.symbol symbol=Reader.read source="private read(): string" type=() => string

}
"#, r#"
/// @diagnostic.error id=interface-member-visibility message="interface members are always public"
/// @diagnostic.label line=3 column=13 span="read" line_source="private read(): string;"
"#);
}

/// An interface field rejects a written visibility modifier.
#[test]
fn test_interface_field_rejects_a_visibility_modifier() {
    let session = TestSession::single(
        r#"
interface Reader {
    protected source: string;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_definitions(),
        r#"
=== annotated ===
interface Reader {
    protected source: string;
}

=== dir ===
interface Reader {
/// @generic.template symbol=Reader parameters=(this: Reader)
/// @type.symbol symbol=Reader type=Reader
/// @definition.interface symbol=Reader template=(this: Reader)
/// @definition.where symbol=Reader relation=satisfies left=this right=Reader
/// @definition.field symbol=Reader.source source="protected source: string" key=source type=string

    protected source: string;
    /// @type.symbol symbol=Reader.source source="protected source: string" type=string

}
"#,
        r#"
/// @diagnostic.error id=interface-member-visibility message="interface members are always public"
/// @diagnostic.label line=3 column=15 span="source" line_source="protected source: string;"
"#,
    );
}
