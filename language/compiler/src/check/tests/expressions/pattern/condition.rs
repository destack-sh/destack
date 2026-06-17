use crate::tests::{DirRows, TestSession};

#[test]
fn test_if_let_pattern_binds_then_branch() {
    let session = TestSession::single(
        r#"
declare const pair: (int32, string) | null;

if (let (count, label) = pair) {
    count satisfies int32;
    label satisfies string;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const pair: (int32, string) | null;

if (let (count, label) = pair) {
    count satisfies int32;
    label satisfies string;
}

=== checked ===
declare const pair: (int32, string) | null;
/// @type.symbol symbol=pair source=pair type=(int32, string) | null

if (let (count, label) = pair) {
/// @type.node type=void | void
/// @type.symbol symbol=count source=count type=int32
/// @type.symbol symbol=label source=label type=string
/// @resolution.pattern source="(count, label)" kind=tuple fields=[0: count, 1: label]
/// @resolution.pattern source=count kind=binding target=count
/// @resolution.pattern source=label kind=binding target=label
/// @type.node source=pair type=(int32, string) | null
/// @resolution.name source=pair target=pair

    count satisfies int32;
    /// @type.node source="count satisfies int32" type=int32
    /// @type.node source=count type=int32
    /// @resolution.name source=count target=count

    label satisfies string;
    /// @type.node source="label satisfies string" type=string
    /// @type.node source=label type=string
    /// @resolution.name source=label target=label

}
"#,
    );
}

#[test]
fn test_if_let_object_pattern_binds_then_branch() {
    let session = TestSession::single(
        r#"
declare const config: { enabled: boolean; retries: int32 } | null;

if (let { enabled, retries } = config) {
    enabled satisfies boolean;
    retries satisfies int32;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const config: { enabled: boolean; retries: int32 } | null;

if (let { enabled, retries } = config) {
    enabled satisfies boolean;
    retries satisfies int32;
}

=== checked ===
declare const config: { enabled: boolean; retries: int32 } | null;
/// @type.symbol symbol=config source=config type={ enabled: boolean; retries: int32 } | null

if (let { enabled, retries } = config) {
/// @type.node type=void | void
/// @type.symbol symbol=enabled source=enabled type=boolean
/// @type.symbol symbol=retries source=retries type=int32
/// @resolution.pattern source="{ enabled, retries }" kind=object fields=[enabled, retries]
/// @resolution.pattern source=enabled kind=binding target=enabled
/// @resolution.pattern source=retries kind=binding target=retries
/// @type.node source=config type={ enabled: boolean; retries: int32 } | null
/// @resolution.name source=config target=config

    enabled satisfies boolean;
    /// @type.node source="enabled satisfies boolean" type=boolean
    /// @type.node source=enabled type=boolean
    /// @resolution.name source=enabled target=enabled

    retries satisfies int32;
    /// @type.node source="retries satisfies int32" type=int32
    /// @type.node source=retries type=int32
    /// @resolution.name source=retries target=retries

}
"#,
    );
}

#[test]
fn test_if_condition_chain_binds_later_operands() {
    let session = TestSession::single(
        r#"
declare const ready: boolean;
declare const pair: (int32, string) | null;

if (ready && let (count, label) = pair && count > 0) {
    label satisfies string;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const ready: boolean;
declare const pair: (int32, string) | null;

if (ready && let (count, label) = pair && count > 0) {
    label satisfies string;
}

=== checked ===
declare const ready: boolean;
/// @type.symbol symbol=ready source=ready type=boolean
declare const pair: (int32, string) | null;
/// @type.symbol symbol=pair source=pair type=(int32, string) | null

if (ready && let (count, label) = pair && count > 0) {
/// @type.node type=void | void
/// @type.node source=ready type=boolean
/// @resolution.name source=ready target=ready
/// @type.symbol symbol=count source=count type=int32
/// @type.symbol symbol=label source=label type=string
/// @resolution.pattern source="(count, label)" kind=tuple fields=[0: count, 1: label]
/// @resolution.pattern source=count kind=binding target=count
/// @resolution.pattern source=label kind=binding target=label
/// @type.node source=pair type=(int32, string) | null
/// @resolution.name source=pair target=pair
/// @type.node source="count > 0" type=boolean
/// @type.node source=count type=int32
/// @resolution.name source=count target=count
/// @type.node source=0 type=0

    label satisfies string;
    /// @type.node source="label satisfies string" type=string
    /// @type.node source=label type=string
    /// @resolution.name source=label target=label

}
"#,
    );
}

#[test]
fn test_if_let_literal_pattern_narrows_source() {
    let session = TestSession::single(
        r#"
declare const status: "ready" | "error";

if (let "ready" = status) {
    status satisfies "ready";
} else {
    status satisfies "error";
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const status: "ready" | "error";

if (let "ready" = status) {
    status satisfies "ready";
} else {
    status satisfies "error";
}

=== checked ===
declare const status: "ready" | "error";
/// @type.symbol symbol=status source=status type="ready" | "error"

if (let "ready" = status) {
/// @type.node type=void | void
/// @type.node source="\"ready\"" type="ready"
/// @resolution.pattern source="\"ready\"" kind=literal value="ready"
/// @type.node source=status type="ready" | "error"
/// @resolution.name source=status target=status

    status satisfies "ready";
    /// @type.node source="status satisfies \"ready\"" type="ready"
    /// @type.node source=status type="ready"
    /// @resolution.name source=status target=status

} else {
    status satisfies "error";
    /// @type.node source="status satisfies \"error\"" type="error"
    /// @type.node source=status type="error"
    /// @resolution.name source=status target=status

}
"#,
    );
}

#[test]
fn test_if_let_union_pattern_narrows_covered_literals() {
    let session = TestSession::single(
        r#"
declare const value: 1 | 2 | 3;

if (let 1 | 2 = value) {
    value satisfies 1 | 2;
} else {
    value satisfies 3;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const value: 1 | 2 | 3;

if (let 1 | 2 = value) {
    value satisfies 1 | 2;
} else {
    value satisfies 3;
}

=== checked ===
declare const value: 1 | 2 | 3;
/// @type.symbol symbol=value source=value type=1 | 2 | 3

if (let 1 | 2 = value) {
/// @type.node type=void | void
/// @type.node source=1 type=1
/// @type.node source=2 type=2
/// @resolution.pattern source="1 | 2" kind=union alternatives=[pattern, pattern]
/// @resolution.pattern source=1 kind=literal value=1
/// @resolution.pattern source=2 kind=literal value=2
/// @type.node source=value type=1 | 2 | 3
/// @resolution.name source=value target=value

    value satisfies 1 | 2;
    /// @type.node source="value satisfies 1 | 2" type=1 | 2
    /// @type.node source=value type=1 | 2
    /// @resolution.name source=value target=value

} else {
    value satisfies 3;
    /// @type.node source="value satisfies 3" type=3
    /// @type.node source=value type=3
    /// @resolution.name source=value target=value

}
"#,
    );
}

#[test]
fn test_condition_chain_allows_binding_after_boolean_operand() {
    let session = TestSession::single(
        r#"
declare const ready: boolean;
declare const user: { name: string } | null;

if (ready && let { name } = user) {
    name satisfies string;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const ready: boolean;
declare const user: { name: string } | null;

if (ready && let { name } = user) {
    name satisfies string;
}

=== checked ===
declare const ready: boolean;
/// @type.symbol symbol=ready source=ready type=boolean
declare const user: { name: string } | null;
/// @type.symbol symbol=user source=user type={ name: string } | null

if (ready && let { name } = user) {
/// @type.node type=void | void
/// @type.node source=ready type=boolean
/// @resolution.name source=ready target=ready
/// @type.symbol symbol=name source=name type=string
/// @resolution.pattern source="{ name }" kind=object fields=[name]
/// @resolution.pattern source=name kind=binding target=name
/// @type.node source=user type={ name: string } | null
/// @resolution.name source=user target=user

    name satisfies string;
    /// @type.node source="name satisfies string" type=string
    /// @type.node source=name type=string
    /// @resolution.name source=name target=name

}
"#,
    );
}

#[test]
fn test_condition_chain_allows_boolean_operand_after_binding() {
    let session = TestSession::single(
        r#"
declare const user: { name: string } | null;

if (let { name } = user && name.length > 0) {
    name satisfies string;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const user: { name: string } | null;

if (let { name } = user && name.length > 0) {
    name satisfies string;
}

=== checked ===
declare const user: { name: string } | null;
/// @type.symbol symbol=user source=user type={ name: string } | null

if (let { name } = user && name.length > 0) {
/// @type.node type=void | void
/// @type.symbol symbol=name source=name type=string
/// @resolution.pattern source="{ name }" kind=object fields=[name]
/// @resolution.pattern source=name kind=binding target=name
/// @type.node source=user type={ name: string } | null
/// @resolution.name source=user target=user
/// @type.node source="name.length > 0" type=boolean
/// @type.node source=name.length type=usize
/// @resolution.member source=name.length receiver=string kind=field key=length
/// @type.node source=0 type=0

    name satisfies string;
    /// @type.node source="name satisfies string" type=string
    /// @type.node source=name type=string
    /// @resolution.name source=name target=name

}
"#,
    );
}

#[test]
fn test_condition_chain_allows_multiple_bindings() {
    let session = TestSession::single(
        r#"
declare const user: { name: string } | null;
declare const point: { x: int32 } | null;

if (let { name } = user && let { x } = point) {
    name satisfies string;
    x satisfies int32;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const user: { name: string } | null;
declare const point: { x: int32 } | null;

if (let { name } = user && let { x } = point) {
    name satisfies string;
    x satisfies int32;
}

=== checked ===
declare const user: { name: string } | null;
/// @type.symbol symbol=user source=user type={ name: string } | null
declare const point: { x: int32 } | null;
/// @type.symbol symbol=point source=point type={ x: int32 } | null

if (let { name } = user && let { x } = point) {
/// @type.node type=void | void
/// @type.symbol symbol=name source=name type=string
/// @resolution.pattern source="{ name }" kind=object fields=[name]
/// @type.node source=user type={ name: string } | null
/// @resolution.name source=user target=user
/// @type.symbol symbol=x source=x type=int32
/// @resolution.pattern source="{ x }" kind=object fields=[x]
/// @type.node source=point type={ x: int32 } | null
/// @resolution.name source=point target=point

    name satisfies string;
    /// @type.node source="name satisfies string" type=string
    /// @type.node source=name type=string
    /// @resolution.name source=name target=name

    x satisfies int32;
    /// @type.node source="x satisfies int32" type=int32
    /// @type.node source=x type=int32
    /// @resolution.name source=x target=x

}
"#,
    );
}

#[test]
fn test_condition_chain_else_branch_cannot_see_binding() {
    let session = TestSession::single(
        r#"
declare const user: { name: string } | null;

if (let { name } = user) {
    name satisfies string;
} else {
    name;
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const user: { name: string } | null;

if (let { name } = user) {
    name satisfies string;
} else {
    name;
}

=== checked ===
declare const user: { name: string } | null;
/// @type.symbol symbol=user source=user type={ name: string } | null

if (let { name } = user) {
/// @type.node type=void | void
/// @type.symbol symbol=name source=name type=string
/// @resolution.pattern source="{ name }" kind=object fields=[name]
/// @type.node source=user type={ name: string } | null
/// @resolution.name source=user target=user

    name satisfies string;
    /// @type.node source="name satisfies string" type=string
    /// @type.node source=name type=string
    /// @resolution.name source=name target=name

} else {
    name;
    /// @type.node source=name type=<error>

}
"#,
        r#"
/// @diagnostic.error code=EC300 message="cannot find name 'name'"
/// @diagnostic.label line=7 column=5 source=name
"#,
    );
}

#[test]
fn test_condition_chain_without_binding_stays_boolean_expression() {
    let session = TestSession::single(
        r#"
declare const left: boolean;
declare const right: boolean;

if (left && right) {
    left satisfies boolean;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const left: boolean;
declare const right: boolean;

if (left && right) {
    left satisfies boolean;
}

=== checked ===
declare const left: boolean;
/// @type.symbol symbol=left source=left type=boolean
declare const right: boolean;
/// @type.symbol symbol=right source=right type=boolean

if (left && right) {
/// @type.node type=void | void
/// @type.node source="left && right" type=boolean
/// @type.node source=left type=boolean
/// @resolution.name source=left target=left
/// @type.node source=right type=boolean
/// @resolution.name source=right target=right

    left satisfies boolean;
    /// @type.node source="left satisfies boolean" type=boolean
    /// @type.node source=left type=boolean
    /// @resolution.name source=left target=left

}
"#,
    );
}
