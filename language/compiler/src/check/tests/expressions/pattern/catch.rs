use crate::tests::{DirRows, TestSession};

#[test]
fn test_catch_pattern_binds_error_fields() {
    let session = TestSession::single(
        r#"
declare function read(): Result<string, { code: int32; message: string }>;

try {
    read()?
} catch ({ code, message }) {
    code satisfies int32;
    message satisfies string;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare function read(): Result<string, { code: int32; message: string }>;

try {
    read()?
} catch ({ code, message }) {
    code satisfies int32;
    message satisfies string;
}

=== checked ===
declare function read(): Result<string, { code: int32; message: string }>;
/// @type.symbol symbol=read type=() => Result<string, { code: int32; message: string }>

try {
    read()?
    /// @type.node source=read()? type=string
    /// @type.node source=read() type=Result<string, { code: int32; message: string }>
    /// @resolution.name source=read target=read
    /// @resolution.call source=read() parameters=() return=Result<string, { code: int32; message: string }> kind=symbol target=read

} catch ({ code, message }) {
/// @type.symbol symbol=code source=code type=int32
/// @type.symbol symbol=message source=message type=string
/// @resolution.pattern source="{ code, message }" kind=object fields=[code, message]

    code satisfies int32;
    /// @type.node source="code satisfies int32" type=int32
    /// @type.node source=code type=int32
    /// @resolution.name source=code target=code

    message satisfies string;
    /// @type.node source="message satisfies string" type=string
    /// @type.node source=message type=string
    /// @resolution.name source=message target=message

}
"#,
    );
}

#[test]
fn test_catch_without_binding_is_allowed() {
    let session = TestSession::single(
        r#"
declare function read(): Result<string, string>;

try {
    read()?
} catch {
    const handled = true;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare function read(): Result<string, string>;

try {
    read()?
} catch {
    const handled: true = true;
}

=== checked ===
declare function read(): Result<string, string>;
/// @type.symbol symbol=read type=() => Result<string, string>

try {
    read()?
    /// @type.node source=read()? type=string
    /// @type.node source=read() type=Result<string, string>
    /// @resolution.name source=read target=read
    /// @resolution.call source=read() parameters=() return=Result<string, string> kind=symbol target=read

} catch {
    const handled = true;
    /// @type.symbol symbol=handled source=handled type=true
    /// @type.node source=true type=true

}
"#,
    );
}

#[test]
fn test_catch_identifier_binding_reads_error_type() {
    let session = TestSession::single(
        r#"
declare function read(): Result<string, { message: string }>;

try {
    read()?
} catch (error) {
    error.message satisfies string;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare function read(): Result<string, { message: string }>;

try {
    read()?
} catch (error) {
    error.message satisfies string;
}

=== checked ===
declare function read(): Result<string, { message: string }>;
/// @type.symbol symbol=read type=() => Result<string, { message: string }>

try {
    read()?
    /// @type.node source=read()? type=string
    /// @type.node source=read() type=Result<string, { message: string }>
    /// @resolution.name source=read target=read
    /// @resolution.call source=read() parameters=() return=Result<string, { message: string }> kind=symbol target=read

} catch (error) {
/// @type.symbol symbol=error source=error type={ message: string }
/// @resolution.pattern source=error kind=binding target=error

    error.message satisfies string;
    /// @type.node source="error.message satisfies string" type=string
    /// @type.node source=error.message type=string
    /// @type.node source=error type={ message: string }
    /// @resolution.name source=error target=error
    /// @resolution.member source=error.message receiver={ message: string } kind=field key=message

}
"#,
    );
}

#[test]
fn test_catch_rejects_refutable_pattern() {
    let session = TestSession::single(
        r#"
declare function read(): Result<string, "missing" | "denied">;

try {
    read()?
} catch ("missing") {
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare function read(): Result<string, "missing" | "denied">;

try {
    read()?
} catch ("missing") {
}

=== checked ===
declare function read(): Result<string, "missing" | "denied">;
/// @type.symbol symbol=read type=() => Result<string, "missing" | "denied">

try {
    read()?
    /// @type.node source=read()? type=string
    /// @type.node source=read() type=Result<string, "missing" | "denied">
    /// @resolution.name source=read target=read
    /// @resolution.call source=read() parameters=() return=Result<string, "missing" | "denied"> kind=symbol target=read

} catch ("missing") {
/// @type.node source="\"missing\"" type="missing"
/// @resolution.pattern source="\"missing\"" kind=literal value="missing"

}
"#,
        r#"
/// @diagnostic.error code=EC437 message="catch pattern must be irrefutable: '\"denied\"' is not covered"
/// @diagnostic.label line=6 column=10 source="\"missing\""
/// @diagnostic.help message="catch bindings must handle every failure value"
"#,
    );
}
