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
/// @type.symbol symbol=read type=(Result<int32, Cancelled>) => int32
/// @type.symbol symbol=read.value source="value: Result<int32, Cancelled>" type=Result<int32, Cancelled>
/// @resolution.name source=Result target=Result
/// @resolution.name source=Cancelled target=Cancelled

    try {
        value?
        /// @resolution.name source=value target=read.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=read.value
        /// @resolution.residual source=value? target=try residual=TryResidual<Result<int32, Cancelled>> branch="branch(parameters=(), arguments=(), return=ControlFlow<Result<never, Cancelled>, int32>)"
        /// @generic.instantiation id="branch<int32, Cancelled>" template=branch arguments=(int32, Cancelled)

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
/// @generic.instance id="Err<{ code: int32; message: string }>" template=Err arguments=({ code: int32; message: string })
/// @generic.instance id="Result<string, { code: int32; message: string }>" template=Result arguments=(string, { code: int32; message: string })
/// @generic.instance id=Ok<string> template=Ok arguments=(string)
/// @resolution.name source=Result target=Result
/// @type.symbol symbol=read.code source="code: int32" type=int32
/// @type.symbol symbol=read.message source="message: string" type=string

try {
/// @type.node type=string | void

    read()?
    /// @type.node source=read type=() => Result<string, { code: int32; message: string }>
    /// @type.node source=read() type=Result<string, { code: int32; message: string }>
    /// @type.node source=read()? type=string
    /// @resolution.name source=read target=read
    /// @resolution.call source=read() parameters=() return=Result<string, { code: int32; message: string }> kind=symbol target=read
    /// @resolution.residual source=read()? target=try residual=TryResidual<Result<string, { code: int32; message: string }>> branch="branch(parameters=(), arguments=(), return=ControlFlow<Result<never, { code: int32; message: string }>, string>)"
    /// @generic.instantiation id="branch<string, { code: int32; message: string }>" template=branch arguments=(string, { code: int32; message: string })
    /// @generic.instance id="Break<Result<never, { code: int32; message: string }>>" template=Break arguments=(Result<never, { code: int32; message: string }>)
    /// @generic.instance id="ControlFlow<Result<never, { code: int32; message: string }>, string>" template=ControlFlow arguments=(Result<never, { code: int32; message: string }>, string)
    /// @generic.instance id="Result<never, { code: int32; message: string }>" template=Result arguments=(never, { code: int32; message: string })
    /// @generic.instance id="branch<string, { code: int32; message: string }>" template=branch arguments=(string, { code: int32; message: string })
    /// @generic.instance id="break<Result<never, { code: int32; message: string }>, string>" template=break arguments=(Result<never, { code: int32; message: string }>, string)
    /// @generic.instance id="continue<Result<never, { code: int32; message: string }>, string>" template=continue arguments=(Result<never, { code: int32; message: string }>, string)
    /// @generic.instance id="err#1<never, { code: int32; message: string }>" template=err#1 arguments=(never, { code: int32; message: string })
    /// @generic.instance id=Continue<string> template=Continue arguments=(string)
    /// @generic.instance id=Ok<never> template=Ok arguments=(never)

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
/// @generic.instance id="Result<string, string>" template=Result arguments=(string, string)
/// @generic.instance id=Err<string> template=Err arguments=(string)
/// @generic.instance id=Ok<string> template=Ok arguments=(string)
/// @resolution.name source=Result target=Result

try {
/// @type.node type=string | void

    read()?
    /// @type.node source=read type=() => Result<string, string>
    /// @type.node source=read() type=Result<string, string>
    /// @type.node source=read()? type=string
    /// @resolution.name source=read target=read
    /// @resolution.call source=read() parameters=() return=Result<string, string> kind=symbol target=read
    /// @resolution.residual source=read()? target=try residual=TryResidual<Result<string, string>> branch="branch(parameters=(), arguments=(), return=ControlFlow<Result<never, string>, string>)"
    /// @generic.instantiation id="branch<string, string>" template=branch arguments=(string, string)
    /// @generic.instance id="Break<Result<never, string>>" template=Break arguments=(Result<never, string>)
    /// @generic.instance id="ControlFlow<Result<never, string>, string>" template=ControlFlow arguments=(Result<never, string>, string)
    /// @generic.instance id="Result<never, string>" template=Result arguments=(never, string)
    /// @generic.instance id="branch<string, string>" template=branch arguments=(string, string)
    /// @generic.instance id="break<Result<never, string>, string>" template=break arguments=(Result<never, string>, string)
    /// @generic.instance id="continue<Result<never, string>, string>" template=continue arguments=(Result<never, string>, string)
    /// @generic.instance id="err#1<never, string>" template=err#1 arguments=(never, string)
    /// @generic.instance id=Continue<string> template=Continue arguments=(string)
    /// @generic.instance id=Ok<never> template=Ok arguments=(never)

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
/// @generic.instance id="Err<{ message: string }>" template=Err arguments=({ message: string })
/// @generic.instance id="Result<string, { message: string }>" template=Result arguments=(string, { message: string })
/// @generic.instance id=Ok<string> template=Ok arguments=(string)
/// @resolution.name source=Result target=Result
/// @type.symbol symbol=read.message source="message: string" type=string

try {
/// @type.node type=string | void

    read()?
    /// @type.node source=read type=() => Result<string, { message: string }>
    /// @type.node source=read() type=Result<string, { message: string }>
    /// @type.node source=read()? type=string
    /// @resolution.name source=read target=read
    /// @resolution.call source=read() parameters=() return=Result<string, { message: string }> kind=symbol target=read
    /// @resolution.residual source=read()? target=try residual=TryResidual<Result<string, { message: string }>> branch="branch(parameters=(), arguments=(), return=ControlFlow<Result<never, { message: string }>, string>)"
    /// @generic.instantiation id="branch<string, { message: string }>" template=branch arguments=(string, { message: string })
    /// @generic.instance id="Break<Result<never, { message: string }>>" template=Break arguments=(Result<never, { message: string }>)
    /// @generic.instance id="ControlFlow<Result<never, { message: string }>, string>" template=ControlFlow arguments=(Result<never, { message: string }>, string)
    /// @generic.instance id="Result<never, { message: string }>" template=Result arguments=(never, { message: string })
    /// @generic.instance id="branch<string, { message: string }>" template=branch arguments=(string, { message: string })
    /// @generic.instance id="break<Result<never, { message: string }>, string>" template=break arguments=(Result<never, { message: string }>, string)
    /// @generic.instance id="continue<Result<never, { message: string }>, string>" template=continue arguments=(Result<never, { message: string }>, string)
    /// @generic.instance id="err#1<never, { message: string }>" template=err#1 arguments=(never, { message: string })
    /// @generic.instance id=Continue<string> template=Continue arguments=(string)
    /// @generic.instance id=Ok<never> template=Ok arguments=(never)

} catch (error) {
/// @type.symbol symbol=error source=error type={ message: string }
/// @resolution.pattern source=error kind=binding target=error

    error.message satisfies string;
    /// @type.node source="error.message satisfies string" type=string
    /// @type.node source=error type={ message: string }
    /// @type.node source=error.message type=string
    /// @resolution.name source=error target=error
    /// @resolution.member source=error.message receiver=TryFailure<Result<string, { message: string }>> type=string kind=field target_receiver=TryFailure<Result<string, { message: string }>> key=message target_type=string
    /// @resolution.place source=error placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=error root=error
    /// @resolution.place source=error.message placement="local" lifetime="managed" access="mutable"
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
/// @resolution.name source=Result target=Result

try {
/// @type.node type=string | void

    read()?
    /// @type.node source=read type=() => Result<string, "missing" | "denied">
    /// @type.node source=read() type=Result<string, "missing" | "denied">
    /// @type.node source=read()? type=string
    /// @resolution.name source=read target=read
    /// @resolution.call source=read() parameters=() return=Result<string, "missing" | "denied"> kind=symbol target=read
    /// @resolution.residual source=read()? target=try residual=TryResidual<Result<string, "missing" | "denied">> branch="branch(parameters=(), arguments=(), return=ControlFlow<Result<never, \"missing\" | \"denied\">, string>)"
    /// @generic.instantiation id="branch<string, \"missing\" | \"denied\">" template=branch arguments=(string, "missing" | "denied")

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
