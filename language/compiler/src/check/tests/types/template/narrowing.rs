use crate::tests::{DirRows, TestSession};

#[test]
fn test_nullish_guard_preserves_template_literal_constraint() {
    let session = TestSession::single(
        r#"
type Route = `api:${string}`;

declare const route: Route | undefined;

if (route != undefined) {
    route satisfies Route;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Route = `api:${string}`;

declare const route: Route | undefined;

if (route != undefined) {
    route satisfies Route;
}

=== checked ===
type Route = `api:${string}`;
/// @type.symbol symbol=Route source="type Route = `api:${string}`" type=`api:${string}`
/// @definition.type symbol=Route source="type Route = `api:${string}`" value=`api:${string}`

declare const route: Route | undefined;
/// @type.symbol symbol=route source=route type=`api:${string}` | undefined
/// @resolution.name source=Route target=Route

if (route != undefined) {
/// @resolution.name source=route target=route

    route satisfies Route;
    /// @resolution.name source=route target=route
    /// @resolution.name source=Route target=Route

}
"#,
    );
}

#[test]
fn test_equality_guard_keeps_template_literal_constraint() {
    let session = TestSession::single(
        r#"
type Route = `api:${string}`;

declare const route: Route;

if (route == "api:users") {
    route satisfies Route;
} else {
    route satisfies Route;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Route = `api:${string}`;

declare const route: Route;

if (route == "api:users") {
    route satisfies Route;
} else {
    route satisfies Route;
}

=== checked ===
type Route = `api:${string}`;
/// @type.symbol symbol=Route source="type Route = `api:${string}`" type=`api:${string}`
/// @definition.type symbol=Route source="type Route = `api:${string}`" value=`api:${string}`

declare const route: Route;
/// @type.symbol symbol=route source=route type=`api:${string}`
/// @resolution.name source=Route target=Route

if (route == "api:users") {
/// @resolution.name source=route target=route

    route satisfies Route;
    /// @resolution.name source=route target=route
    /// @resolution.name source=Route target=Route

} else {
    route satisfies Route;
    /// @resolution.name source=route target=route
    /// @resolution.name source=Route target=Route

}
"#,
    );
}

#[test]
fn test_match_over_template_literal_type_union_is_exhaustive() {
    let session = TestSession::single(
        r#"
type Route = `api:${"users" | "posts"}`;

const route: Route = "api:users";

const section = match (route) {
    "api:users" => "users"
    "api:posts" => "posts"
};

section satisfies "users" | "posts";
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Route = `api:${"users" | "posts"}`;

const route: Route = "api:users" as Route;

const section: "users" | "posts" = match (route) {
    "api:users" => "users"
    "api:posts" => "posts"
};

section satisfies "users" | "posts";

=== checked ===
type Route = `api:${"users" | "posts"}`;
/// @type.symbol symbol=Route source="type Route = `api:${\"users\" | \"posts\"}`" type=`api:${"users" | "posts"}`
/// @definition.type symbol=Route source="type Route = `api:${\"users\" | \"posts\"}`" value=`api:${"users" | "posts"}`

const route: Route = "api:users";
/// @type.symbol symbol=route source=route type=`api:${"users" | "posts"}`
/// @resolution.name source=Route target=Route

const section = match (route) {
/// @type.symbol symbol=section type="users" | "posts"
/// @resolution.name source=route target=route

    "api:users" => "users"
    "api:posts" => "posts"
};

section satisfies "users" | "posts";
/// @resolution.name source=section target=section
"#,
    );
}
