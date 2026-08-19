use crate::tests::{DirRows, TestSession};

#[test]
fn test_reject_catch_binding_shadowing_type() {
    let session = TestSession::single(
        r#"
import { Result } from "destack:error";

struct Cancelled {
    reason: int32;
}

function read(value: Result<int32, Cancelled>): int32 {
    try {
        value?
    } catch (Cancelled) {
        0
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Result } from "destack:error";

struct Cancelled {
    reason: int32;
}

function read(value: Result<int32, Cancelled>): int32 {
    try {
        value?
    } catch (Cancelled) {
        0
    }
}

=== dir ===
import { Result } from "destack:error";

struct Cancelled {
/// @type.symbol symbol=Cancelled type=Cancelled
/// @definition.struct symbol=Cancelled
/// @definition.field symbol=Cancelled.reason source="reason: int32" key=reason type=int32

    reason: int32;
    /// @type.symbol symbol=Cancelled.reason source="reason: int32" type=int32

}

function read(value: Result<int32, Cancelled>): int32 {
/// @type.symbol symbol=read type=(error.result.Result<int32, Cancelled>) => int32
/// @type.symbol symbol=read.value source="value: Result<int32, Cancelled>" type=error.result.Result<int32, Cancelled>
/// @resolution.name source=Result target=error.result.Result
/// @resolution.name source=Cancelled target=Cancelled

    try {
        value?
        /// @resolution.name source=value target=read.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=read.value

    } catch (Cancelled) {
    /// @type.symbol symbol=read.Cancelled source=Cancelled type=Cancelled
    /// @resolution.pattern source=Cancelled kind=binding target=read.Cancelled

        0
    }
}
"#,
        r#"
/// @diagnostic.error id=pattern-shadows-type message="bare pattern 'Cancelled' binds a new variable that shadows a type"
/// @diagnostic.label line=11 column=14 span="Cancelled" line_source="} catch (Cancelled) {"
/// @diagnostic.help message="match values of the type with a nominal pattern like 'Cancelled { }'"
"#,
    );
}

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

    session.assert_dir(
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

=== dir ===
declare function read(): Result<string, { code: int32; message: string }>;
/// @type.symbol symbol=read source="declare function read(): Result<string, { code: int32; message: string }>" type=() => Result<string, { code: int32; message: string }>
/// @generic.instance id="Result<string, { code: int32; message: string }>" template=error.result.Result arguments=(string, { code: int32; message: string })
/// @generic.instance id="error.result.Err<{ code: int32; message: string }>" template=error.result.Err arguments=({ code: int32; message: string })
/// @generic.instance id=error.result.Ok<string> template=error.result.Ok arguments=(string)
/// @resolution.name source=Result target=error.result.Result

try {
/// @type.node type=string | void

    read()?
    /// @type.node source=read type=() => Result<string, { code: int32; message: string }>
    /// @type.node source=read() type=Result<string, { code: int32; message: string }>
    /// @type.node source=read()? type=string
    /// @resolution.name source=read target=read
    /// @resolution.call source=read() parameters=() return=Result<string, { code: int32; message: string }> kind=symbol target=read

} catch ({ code, message }) {
/// @resolution.pattern source={ code, message } kind=object fields={ code, message }
/// @type.symbol symbol=code source=code type=int32
/// @type.symbol symbol=message source=message type=string

    code satisfies int32;
    /// @type.node source="code satisfies int32" type=int32
    /// @type.node source=code type=int32
    /// @resolution.name source=code target=code
    /// @resolution.place source=code placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=code root=code

    message satisfies string;
    /// @type.node source="message satisfies string" type=string
    /// @type.node source=message type=string
    /// @resolution.name source=message target=message
    /// @resolution.place source=message placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=message root=message

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

    session.assert_dir(
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

=== dir ===
declare function read(): Result<string, string>;
/// @type.symbol symbol=read source="declare function read(): Result<string, string>" type=() => Result<string, string>
/// @generic.instance id="Result<string, string>" template=error.result.Result arguments=(string, string)
/// @generic.instance id=error.result.Err<string> template=error.result.Err arguments=(string)
/// @generic.instance id=error.result.Ok<string> template=error.result.Ok arguments=(string)
/// @resolution.name source=Result target=error.result.Result

try {
/// @type.node type=string | void

    read()?
    /// @type.node source=read type=() => Result<string, string>
    /// @type.node source=read() type=Result<string, string>
    /// @type.node source=read()? type=string
    /// @resolution.name source=read target=read
    /// @resolution.call source=read() parameters=() return=Result<string, string> kind=symbol target=read

} catch {
    const handled = true;
    /// @type.symbol symbol=handled source=handled type=true
    /// @resolution.pattern source=handled kind=binding target=handled
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

    session.assert_dir(
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

=== dir ===
declare function read(): Result<string, { message: string }>;
/// @type.symbol symbol=read source="declare function read(): Result<string, { message: string }>" type=() => Result<string, { message: string }>
/// @generic.instance id="Result<string, { message: string }>" template=error.result.Result arguments=(string, { message: string })
/// @generic.instance id="error.result.Err<{ message: string }>" template=error.result.Err arguments=({ message: string })
/// @generic.instance id=error.result.Ok<string> template=error.result.Ok arguments=(string)
/// @resolution.name source=Result target=error.result.Result

try {
/// @type.node type=string | void

    read()?
    /// @type.node source=read type=() => Result<string, { message: string }>
    /// @type.node source=read() type=Result<string, { message: string }>
    /// @type.node source=read()? type=string
    /// @resolution.name source=read target=read
    /// @resolution.call source=read() parameters=() return=Result<string, { message: string }> kind=symbol target=read

} catch (error) {
/// @type.symbol symbol=error source=error type={ message: string }
/// @resolution.pattern source=error kind=binding target=error

    error.message satisfies string;
    /// @type.node source="error.message satisfies string" type=string
    /// @type.node source=error type={ message: string }
    /// @type.node source=error.message type=string
    /// @resolution.name source=error target=error
    /// @resolution.member source=error.message receiver=TryResidual<Result<string, { message: string }>> type=string kind=field target_receiver=TryResidual<Result<string, { message: string }>> key=message target_type=string
    /// @resolution.place source=error placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=error root=error
    /// @resolution.place source=error.message placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=error.message root=error keys=[message]

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

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare function read(): Result<string, "missing" | "denied">;

try {
    read()?
} catch ("missing") {
}

=== dir ===
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

} catch ("missing") {
/// @type.node source="\"missing\"" type="missing"
/// @resolution.pattern source="\"missing\"" kind=literal value="missing"

}
"#,
        r#"
/// @diagnostic.error id=refutable-catch-pattern message="catch pattern must be irrefutable: '\"denied\"' is not covered"
/// @diagnostic.label line=6 column=10 span="\"missing\"" line_source="} catch (\"missing\") {"
/// @diagnostic.help message="catch bindings must handle every failure value"
"#,
    );
}
