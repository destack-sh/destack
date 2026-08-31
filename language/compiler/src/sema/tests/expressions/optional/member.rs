use crate::tests::{DirRows, TestSession};

#[test]
fn test_optional_member_selects_non_nullish_receiver() {
    let session = TestSession::single(
        r#"
function name(user: { name: string } | null): string | undefined {
    return user?.name;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_node_types(),
        r#"
=== annotated ===
function name(user: { name: string } | null): string | undefined {
    return user?.name as string | undefined;
}

=== dir ===
function name(user: { name: string } | null): string | undefined {
/// @type.symbol symbol=name type=({ name: string } | null) => string | undefined
/// @type.symbol symbol=name.user source="user: { name: string } | null" type={ name: string } | null
/// @type.symbol symbol=name.name source="name: string" type=string

    return user?.name;
    /// @type.node source=user?.name type=string
    /// @type.node source=user?.name type=string | undefined
    /// @resolution.name source=user target=name.user
    /// @resolution.member source=user?.name receiver={ name: string } | null type=string kind=field target_receiver={ name: string } | null adjustments=(union.payload({ name: string } | null, { name: string }, { name: string })) key=name target_type=string
    /// @resolution.place source=user placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=user root=name.user
    /// @resolution.place source=user?.name placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=user?.name root=name.user keys=[name]

}
"#,
    );
}

#[test]
fn test_member_reports_nullish_receiver() {
    let session = TestSession::single(
        r#"
function name(user: { name: string } | null): string {
    return user.name;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_node_types(),
        r#"
=== annotated ===
function name(user: { name: string } | null): string {
    return user.name;
}

=== dir ===
function name(user: { name: string } | null): string {
/// @type.symbol symbol=name type=({ name: string } | null) => string
/// @type.symbol symbol=name.user source="user: { name: string } | null" type={ name: string } | null
/// @type.symbol symbol=name.name source="name: string" type=string

    return user.name;
    /// @type.node source=user.name type=string
    /// @resolution.name source=user target=name.user
    /// @resolution.member source=user.name receiver={ name: string } | null type=string kind=field target_receiver={ name: string } | null adjustments=(union.payload({ name: string } | null, { name: string }, { name: string })) key=name target_type=string
    /// @resolution.place source=user placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=user root=name.user
    /// @resolution.place source=user.name placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=user.name root=name.user keys=[name]

}
"#,
        r#"
/// @diagnostic.error id=possibly-nullish message="value is possibly null"
/// @diagnostic.label line=3 column=17 span="name" line_source="return user.name;"
/// @diagnostic.help message="narrow the value with a check or access it with '?.'"
"#,
    );
}
