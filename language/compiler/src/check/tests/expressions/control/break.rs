use crate::tests::{DirRows, TestSession};

#[test]
fn test_nested_function_break_reports_error() {
    let session = TestSession::single(
        r#"
while (true) {
    const stop = () => {
        break;
    };
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
while (true) {
    const stop: () => void = (): void => {
        break;
    };
}

=== checked ===
while (true) {
/// @type.node source=true type=true

    const stop = () => {
    /// @type.symbol symbol=stop source=stop type=Function<(), void>
    /// @type.symbol symbol=symbol1 type=Function<(), void>
    /// @type.node type=Function<(), void>

        break;
        /// @type.node source=break type=never

    };
}
"#,
        r#"
/// @diagnostic.error code=EC402 message="break statement has no target"
/// @diagnostic.label line=4 column=9 span="break" line_source="break;"
/// @diagnostic.warning code=WC402 message="condition is always true"
/// @diagnostic.label line=2 column=8 span="true" line_source="while (true) {"
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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const flag: boolean;

const value: 1 | 2 = loop {
    if (flag) {
        break 1 as 1 | 2;
    }
    break 2 as 1 | 2;
};

=== checked ===
declare const flag: boolean;
/// @type.symbol symbol=flag source=flag type=boolean

const value = loop {
/// @type.symbol symbol=value source=value type=1 | 2
/// @type.node type=1 | 2

    if (flag) {
    /// @type.node source=flag type=boolean
    /// @resolution.name source=flag target=flag

        break 1;
        /// @type.node source="break 1" type=never
        /// @type.node source=1 type=1

    }
    break 2;
    /// @type.node source="break 2" type=never
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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function spin(): never {
    loop {}
}

=== checked ===
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
while (true) {
    break 1;
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
while (true) {
    break 1;
}

=== checked ===
while (true) {
/// @type.node source=true type=true

    break 1;
    /// @type.node source="break 1" type=never
    /// @type.node source=1 type=1

}
"#,
        r#"
/// @diagnostic.error code=EC441 message="break with a value can only target a `loop` or labeled block"
/// @diagnostic.label line=3 column=5 span="break 1" line_source="break 1;"
/// @diagnostic.warning code=WC402 message="condition is always true"
/// @diagnostic.label line=2 column=8 span="true" line_source="while (true) {"
"#,
    );
}

#[test]
fn test_unlabeled_break_skips_labeled_blocks() {
    let session = TestSession::single(
        r#"
outer: {
    break;
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
outer: {
    break;
}

=== checked ===
outer: {
/// @type.node type=void

    break;
    /// @type.node source=break type=never

}
"#,
        r#"
/// @diagnostic.error code=EC402 message="break statement has no target"
/// @diagnostic.label line=3 column=5 span="break" line_source="break;"
"#,
    );
}
