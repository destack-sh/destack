use crate::tests::{DirRows, TestSession};

#[test]
fn test_associated_type_uses_owner_generic_argument() {
    let session = TestSession::single(
        r#"
struct Box<T> {
    type Item = T;
    value: T;
}

declare const value: Box<string>.Item;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_statics(),
        r#"
struct Box<T> {
/// @generic.template symbol=Box parameters=[T]
/// @type.symbol symbol=Box type=Box<T>

    type Item = T;
    /// @type.symbol symbol=Box.Item type=T

    value: T;
    /// @type.symbol symbol=Box.value type=T
}

declare const value: Box<string>.Item;
/// @resolution.name source=Box target=Box
/// @resolution.member source=Box<string>.Item receiver=Box<string> kind=symbol target=Box.Item
/// @generic.application source="Box<string>" id=Box<string>
/// @type.symbol symbol=value type=string

/// @generic.application id=Box<string> symbol=Box arguments=[string]
"#,
    );
}

#[test]
fn test_associated_type_used_as_value_reports_error() {
    let session = TestSession::single(
        r#"
class Packet {
    type Size = uint32;
}

const size = Packet.Size;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
class Packet {
/// @type.symbol symbol=Packet type=Packet

    type Size = uint32;
    /// @type.symbol symbol=Packet.Size type=uint32
}

const size = Packet.Size;
/// @resolution.name source=Packet target=Packet

"#,
        r#"
/// @diagnostic.error code=EC300 message="missing member 'Size'"
/// @diagnostic.label line=6 column=21 source="const size = Packet.Size;"
"#,
    );
}
