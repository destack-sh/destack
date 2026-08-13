use crate::tests::{DirRows, TestSession};

#[test]
fn test_reject_second_call_of_a_once_callable() {
    let session = TestSession::single(
        r#"
import { Function } from "destack:types";

function run(finish: Function<(), void, "once">): void {
    finish();
    finish();
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Function } from "destack:types";

function run(finish: () => void): void {
    finish();
    finish();
}

=== checked ===
import { Function } from "destack:types";

function run(finish: Function<(), void, "once">): void {
/// @type.symbol symbol=run type=(Function<(), void>) => void
/// @type.symbol symbol=run type=(types.function.Function<(), void, "once">) => void
/// @type.symbol symbol=run.finish source="finish: Function<(), void, \"once\">" type=Function<(), void>
/// @resolution.name source=Function target=types.function.Function

    finish();
    /// @resolution.name source=finish target=run.finish
    /// @resolution.call source=finish() parameters=() return=void kind=expression target=expression
    /// @resolution.place source=finish placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=finish root=run.finish

    finish();
    /// @resolution.name source=finish target=run.finish
    /// @resolution.call source=finish() parameters=() return=void kind=expression target=expression
    /// @resolution.place source=finish placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=finish root=run.finish

}

/// @generic.instance id="types.function.Function<(), void, \"once\">" template=types.function.Function arguments=((), void, "once")
"#,
        r#"
/// @diagnostic.error id=use-after-moved message="'finish' is used after being moved"
/// @diagnostic.label line=6 column=5 span="finish" line_source="finish();"
/// @diagnostic.help message="reassign the binding before this use, or copy instead of moving"
"#,
    );
}

#[test]
fn test_call_a_once_callable_through_its_owned_binding() {
    let session = TestSession::single(
        r#"
import { Function } from "destack:types";

function run(finish: Function<(), void, "once">): void {
    const owned = finish;
    owned();
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Function } from "destack:types";

function run(finish: () => void): void {
    const owned: () => void = finish;
    owned();
}

=== checked ===
import { Function } from "destack:types";

function run(finish: Function<(), void, "once">): void {
/// @type.symbol symbol=run type=(Function<(), void>) => void
/// @type.symbol symbol=run type=(types.function.Function<(), void, "once">) => void
/// @type.symbol symbol=run.finish source="finish: Function<(), void, \"once\">" type=Function<(), void>
/// @resolution.name source=Function target=types.function.Function

    const owned = finish;
    /// @type.symbol symbol=run.owned source=owned type=Function<(), void>
    /// @resolution.pattern source=owned kind=binding target=run.owned
    /// @resolution.name source=finish target=run.finish
    /// @resolution.access source=finish root=run.finish

    owned();
    /// @resolution.name source=owned target=run.owned
    /// @resolution.call source=owned() parameters=() return=void kind=expression target=expression
    /// @resolution.place source=owned placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=owned root=run.owned

}

/// @generic.instance id="types.function.Function<(), void, \"once\">" template=types.function.Function arguments=((), void, "once")
"#,
        r#""#,
    );
}

#[test]
fn test_call_a_repeatable_callable_twice() {
    let session = TestSession::single(
        r#"
import { Function } from "destack:types";

function run(step: Function<(), void>): void {
    step();
    step();
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Function } from "destack:types";

function run(step: () => void): void {
    step();
    step();
}

=== checked ===
import { Function } from "destack:types";

function run(step: Function<(), void>): void {
/// @type.symbol symbol=run type=(Function<(), void>) => void
/// @type.symbol symbol=run type=(types.function.Function<(), void>) => void
/// @type.symbol symbol=run.step source="step: Function<(), void>" type=Function<(), void>
/// @resolution.name source=Function target=types.function.Function

    step();
    /// @resolution.name source=step target=run.step
    /// @resolution.call source=step() parameters=() return=void kind=expression target=expression
    /// @resolution.place source=step placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=step root=run.step

    step();
    /// @resolution.name source=step target=run.step
    /// @resolution.call source=step() parameters=() return=void kind=expression target=expression
    /// @resolution.place source=step placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=step root=run.step

}

/// @generic.instance id="types.function.Function<(), void>" template=types.function.Function arguments=((), void)
"#,
        r#""#,
    );
}
