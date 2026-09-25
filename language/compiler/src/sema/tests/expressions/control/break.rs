use crate::tests::{DirRows, TestSession};

#[test]
fn test_nested_function_break_reports_error() {
    let session = TestSession::single(
        r#"
loop {
    const stop = () => {
        break;
    };
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
loop {
    const stop: () => void = (): void => {
        break;
    };
}

=== dir ===
loop {
/// @type.node type=never

    const stop = () => {
    /// @type.symbol symbol=stop source=stop type=Function<(), void, "readonly">
    /// @resolution.pattern source=stop kind=binding target=stop
    /// @type.symbol symbol=symbol1 type=Function<(), void, "readonly">
    /// @type.node type=Function<(), void, "readonly">

        break;
        /// @type.node source=break type=never

    };
}
"#,
        r#"
/// @diagnostic.error id=break-outside-control-target message="break statement has no target"
/// @diagnostic.label line=4 column=9 span="break" line_source="break;"
"#,
    );
}

#[test]
fn test_loop_break_value_types_the_loop() {
    let session = TestSession::single(
        r#"
declare const flag: boolean;

const value = loop {
    if (flag) {
        break 1;
    }
    break 2;
};
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const flag: boolean;

const value: 1 | 2 = loop {
    if (flag) {
        break 1;
    }
    break 2;
};

=== dir ===
declare const flag: boolean;
/// @type.symbol symbol=flag source=flag type=boolean
/// @resolution.pattern source=flag kind=binding target=flag

const value = loop {
/// @type.symbol symbol=value source=value type=1 | 2
/// @resolution.pattern source=value kind=binding target=value
/// @type.node type=1 | 2

    if (flag) {
    /// @type.node source=flag type=boolean
    /// @resolution.name source=flag target=flag
    /// @resolution.place source=flag placement="local" lifetime="static" access="immutable"
    /// @resolution.access source=flag root=flag

        break 1;
        /// @type.node source="break 1" type=never
        /// @resolution.transfer source="break 1" target=loop
        /// @type.node source=1 type=1

    }
    break 2;
    /// @type.node source="break 2" type=never
    /// @resolution.transfer source="break 2" target=loop
    /// @type.node source=2 type=2

};
"#,
        r#"

"#,
    );
}

#[test]
fn test_break_free_loop_types_as_never() {
    let session = TestSession::single(
        r#"
function spin(): never {
    loop {}
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function spin(): never {
    loop {}
}

=== dir ===
function spin(): never {
/// @type.symbol symbol=spin type=() => never

    loop {}
    /// @type.node source="loop {}" type=never

}
"#,
        r#"
"#,
    );
}

#[test]
fn test_break_value_outside_loop_reports_error() {
    let session = TestSession::single(
        r#"
loop {
    break 1;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
loop {
    break 1;
}

=== dir ===
loop {
/// @type.node type=int64

    break 1;
    /// @type.node source="break 1" type=never
    /// @resolution.transfer source="break 1" target=loop
    /// @type.node source=1 type=1

}
"#,
        r#"

"#,
    );
}

#[test]
fn test_labeled_break_selects_target() {
    let session = TestSession::single(
        r#"
outer: loop {
    break outer;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
outer: loop {
    break outer;
}

=== dir ===
outer: loop {
    break outer;
    /// @type.node source="break outer" type=never
    /// @resolution.transfer source="break outer" target=outer

}
"#,
        r#"
"#,
    );
}

#[test]
fn test_labeled_continue_selects_target() {
    let session = TestSession::single(
        r#"
declare const running: boolean;

outer: while (running) {
    continue outer;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const running: boolean;

outer: while (running) {
    continue outer;
}

=== dir ===
declare const running: boolean;
/// @type.symbol symbol=running source=running type=boolean
/// @resolution.pattern source=running kind=binding target=running

outer: while (running) {
/// @type.node source=running type=boolean
/// @resolution.name source=running target=running
/// @resolution.place source=running placement="local" lifetime="static" access="immutable"
/// @resolution.access source=running root=running

    continue outer;
    /// @type.node source="continue outer" type=never
    /// @resolution.transfer source="continue outer" target=outer

}
"#,
        r#"
"#,
    );
}
