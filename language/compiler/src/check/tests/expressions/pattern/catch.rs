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
/// @type.symbol symbol=read source="declare function read(): Result<string, { code: int32; message: string }>" type=() => Result<string, { code: int32; message: string }>
/// @resolution.name source=Result target=error.result.Result

try {
/// @type.node type=string | void

    read()?
    /// @type.node source=read type=() => Result<string, { code: int32; message: string }>
    /// @type.node source=read() type=Result<string, { code: int32; message: string }>
    /// @type.node source=read()? type=string
    /// @resolution.name source=read target=read
    /// @resolution.call source=read() parameters=() return=Result<string, { code: int32; message: string }> kind=symbol target=read
    /// @generic.instance source=read id="Result<string, { code: int32; message: string }>"
    /// @generic.instance source=read() id="Result<string, { code: int32; message: string }>"

} catch ({ code, message }) {
/// @resolution.pattern source={ code, message } kind=object fields={ code, message }
/// @type.symbol symbol=code source=code type=int32
/// @type.symbol symbol=message source=message type=string

    code satisfies int32;
    /// @type.node source="code satisfies int32" type=int32
    /// @type.node source=code type=int32
    /// @resolution.name source=code target=code

    message satisfies string;
    /// @type.node source="message satisfies string" type=string
    /// @type.node source=message type=string
    /// @resolution.name source=message target=message

}

/// @generic.instance id="Result<string, { code: int32; message: string }>" template=error.result.Result arguments=(string, { code: int32; message: string })
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
/// @type.symbol symbol=read source="declare function read(): Result<string, string>" type=() => Result<string, string>
/// @resolution.name source=Result target=error.result.Result

try {
/// @type.node type=string | void

    read()?
    /// @type.node source=read type=() => Result<string, string>
    /// @type.node source=read() type=Result<string, string>
    /// @type.node source=read()? type=string
    /// @resolution.name source=read target=read
    /// @resolution.call source=read() parameters=() return=Result<string, string> kind=symbol target=read
    /// @generic.instance source=read id="Result<string, string>"
    /// @generic.instance source=read() id="Result<string, string>"

} catch {
    const handled = true;
    /// @type.symbol symbol=handled source=handled type=true
    /// @type.node source=true type=true

}

/// @generic.instance id="Result<string, string>" template=error.result.Result arguments=(string, string)
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
/// @type.symbol symbol=read source="declare function read(): Result<string, { message: string }>" type=() => Result<string, { message: string }>
/// @resolution.name source=Result target=error.result.Result

try {
/// @type.node type=string | void

    read()?
    /// @type.node source=read type=() => Result<string, { message: string }>
    /// @type.node source=read() type=Result<string, { message: string }>
    /// @type.node source=read()? type=string
    /// @resolution.name source=read target=read
    /// @resolution.call source=read() parameters=() return=Result<string, { message: string }> kind=symbol target=read
    /// @generic.instance source=read id="Result<string, { message: string }>"
    /// @generic.instance source=read() id="Result<string, { message: string }>"

} catch (error) {
/// @type.symbol symbol=error source=error type={ message: string }
/// @resolution.pattern source=error kind=binding target=error

    error.message satisfies string;
    /// @type.node source="error.message satisfies string" type=string
    /// @type.node source=error type={ message: string }
    /// @type.node source=error.message type=string
    /// @resolution.name source=error target=error
    /// @resolution.member source=error.message receiver={ message: string } kind=field key=message

}

/// @generic.instance id="Result<string, { message: string }>" template=error.result.Result arguments=(string, { message: string })
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
/// @type.symbol symbol=read source="declare function read(): Result<string, \"missing\" | \"denied\">" type=() => Result<string, "missing" | "denied">
/// @resolution.name source=Result target=error.result.Result

try {
/// @type.node type=string | void

    read()?
    /// @type.node source=read type=() => Result<string, "missing" | "denied">
    /// @type.node source=read() type=Result<string, "missing" | "denied">
    /// @type.node source=read()? type=string
    /// @resolution.name source=read target=read
    /// @resolution.call source=read() parameters=() return=Result<string, "missing" | "denied"> kind=symbol target=read
    /// @generic.instance source=read id="Result<string, \"missing\" | \"denied\">"
    /// @generic.instance source=read() id="Result<string, \"missing\" | \"denied\">"

} catch ("missing") {
/// @type.node source="\"missing\"" type="missing"
/// @resolution.pattern source="\"missing\"" kind=literal value="missing"

}

/// @generic.instance id="Result<string, \"missing\" | \"denied\">" template=error.result.Result arguments=(string, "missing" | "denied")
"#,
        r#"
/// @diagnostic.error code=EC437 message="catch pattern must be irrefutable: '\"denied\"' is not covered"
/// @diagnostic.label line=6 column=10 span="\"missing\"" line_source="} catch (\"missing\") {"
"#,
    );
}
