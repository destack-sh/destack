use crate::tests::{DirRows, TestSession};

/// Narrow by equality against a singleton widened behind an `as` assertion.
#[test]
fn test_equality_narrows_through_an_as_assertion() {
    let session = TestSession::single(
        r#"
function first(value: int32 | undefined): int32 {
    if (value !== (undefined as int32 | undefined)) {
        return value;
    }
    return 0;
}
"#,
    );

    session.assert_dir("main.ds", DirRows::checked().with_node_types(), r#"
=== annotated ===
function first(value: int32 | undefined): int32 {
    if (value !== (undefined as int32 | undefined)) {
        return value;
    }
    return 0;
}

=== dir ===
function first(value: int32 | undefined): int32 {
/// @type.symbol symbol=first type=(int32 | undefined) => int32
/// @type.symbol symbol=first.value source="value: int32 | undefined" type=int32 | undefined

    if (value !== (undefined as int32 | undefined)) {
    /// @type.node source="value !== (undefined as int32 | undefined)" type=boolean
    /// @resolution.name source=value target=first.value
    /// @resolution.operator source="value !== (undefined as int32 | undefined)" type=boolean operator="!==" kind=builtin operands=[value as int32 | undefined families=(integer | undefined), undefined as int32 | undefined as int32 | undefined families=(integer | undefined)]
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=first.value
    /// @type.node source="undefined as int32 | undefined" type=int32 | undefined
    /// @type.node source=undefined type=undefined

        return value;
        /// @resolution.name source=value target=first.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=first.value

    }
    return 0;
    /// @type.node source=0 type=0

}
"#);
}

/// Narrow by equality against a singleton kept by a `satisfies` assertion.
#[test]
fn test_equality_narrows_through_a_satisfies_assertion() {
    let session = TestSession::single(
        r#"
function first(value: int32 | undefined): int32 {
    if (value !== (undefined satisfies int32 | undefined)) {
        return value;
    }
    return 0;
}
"#,
    );

    session.assert_dir("main.ds", DirRows::checked().with_node_types(), r#"
=== annotated ===
function first(value: int32 | undefined): int32 {
    if (value !== ((undefined satisfies int32 | undefined) as int32 | undefined)) {
        return value;
    }
    return 0;
}

=== dir ===
function first(value: int32 | undefined): int32 {
/// @type.symbol symbol=first type=(int32 | undefined) => int32
/// @type.symbol symbol=first.value source="value: int32 | undefined" type=int32 | undefined

    if (value !== (undefined satisfies int32 | undefined)) {
    /// @type.node source="value !== (undefined satisfies int32 | undefined)" type=boolean
    /// @resolution.name source=value target=first.value
    /// @resolution.operator source="value !== (undefined satisfies int32 | undefined)" type=boolean operator="!==" kind=builtin operands=[value as int32 | undefined families=(integer | undefined), undefined satisfies int32 | undefined as int32 | undefined families=(integer | undefined)]
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=first.value
    /// @type.node source="undefined satisfies int32 | undefined" type=undefined
    /// @type.node source=undefined type=undefined

        return value;
        /// @resolution.name source=value target=first.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=first.value

    }
    return 0;
    /// @type.node source=0 type=0

}
"#);
}

/// Keep the flow unnarrowed when the asserted comparand is no singleton.
#[test]
fn test_equality_ignores_a_widened_non_singleton_assertion() {
    let session = TestSession::single(
        r#"
function keep(value: int32 | undefined, other: int32 | undefined): int32 {
    if (value !== (other as int32 | undefined)) {
        return value;
    }
    return 0;
}
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
function keep(value: int32 | undefined, other: int32 | undefined): int32 {
    if (value !== (other as int32 | undefined)) {
        return value;
    }
    return 0;
}

=== dir ===
function keep(value: int32 | undefined, other: int32 | undefined): int32 {
/// @type.symbol symbol=keep type=(int32 | undefined, int32 | undefined) => int32
/// @type.symbol symbol=keep.value source="value: int32 | undefined" type=int32 | undefined
/// @type.symbol symbol=keep.other source="other: int32 | undefined" type=int32 | undefined

    if (value !== (other as int32 | undefined)) {
    /// @resolution.name source=value target=keep.value
    /// @resolution.operator source="value !== (other as int32 | undefined)" type=boolean operator="!==" kind=builtin operands=[value as int32 | undefined families=(integer | undefined), other as int32 | undefined as int32 | undefined families=(integer | undefined)]
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=keep.value
    /// @resolution.name source=other target=keep.other
    /// @resolution.place source=other placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=other root=keep.other

        return value;
        /// @resolution.name source=value target=keep.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=keep.value

    }
    return 0;
}
"#,
        r#"
/// @diagnostic.warning id=redundant-cast message="cast to 'int32 | undefined' has no effect"
/// @diagnostic.label line=3 column=26 span="as" line_source="if (value !== (other as int32 | undefined)) {"
/// @diagnostic.suggestion message="remove the cast" applicability=automatic patched="if (value !== (other int32 | undefined)) {"
/// @diagnostic.error id=return-not-assignable message="type 'int32 | undefined' is not assignable to the declared result type 'int32'"
/// @diagnostic.label line=4 column=16 span="value" line_source="return value;"
/// @diagnostic.note message="expected 'int32', found 'undefined'"
"#,
    );
}

/// Narrow a union by a function-type `is` guard on both branches.
#[test]
fn test_function_type_guard_narrows_both_branches() {
    let session = TestSession::single(
        r#"
function render(message: string | (() => string) | undefined): string {
    if (message is () => string) {
        return message();
    }
    return message ?? "fallback";
}
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
function render(message: string | (() => string) | undefined): string {
    if (message is () => string) {
        return message();
    }
    return message ?? "fallback";
}

=== dir ===
function render(message: string | (() => string) | undefined): string {
/// @type.symbol symbol=render type=(string | Function<(), string> | undefined) => string
/// @type.symbol symbol=render.message source="message: string | (() => string) | undefined" type=string | Function<(), string> | undefined

    if (message is () => string) {
    /// @resolution.name source=message target=render.message
    /// @resolution.guard source="message is () => string" kind=is value=string | Function<(), string> | undefined target=Function<(), string> predicate="string | Function<(), string> | undefined is type(Function<(), string>)" narrowed=Narrow<string | Function<(), string> | undefined, Function<(), string>>
    /// @resolution.place source=message placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=message root=render.message

        return message();
        /// @resolution.name source=message target=render.message
        /// @resolution.call source=message() parameters=() return=string kind=expression target=expression
        /// @resolution.place source=message placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=message root=render.message

    }
    return message ?? "fallback";
    /// @resolution.name source=message target=render.message
    /// @resolution.operator source="message ?? \"fallback\"" type=string operator="??" kind=builtin operands=[message as string | undefined families=(string | undefined), "fallback" as "fallback" families=(string)]
    /// @resolution.place source=message placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=message root=render.message

}
"#, "");
}
