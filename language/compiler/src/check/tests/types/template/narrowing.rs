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
/// @type.symbol symbol=route source=route type=Route | undefined
/// @resolution.pattern source=route kind=binding target=route
/// @resolution.name source=Route target=Route

if (route != undefined) {
/// @resolution.name source=route target=route
/// @resolution.operator source="route != undefined" type=boolean operator="!=" kind=builtin operands=[route as Route | undefined families=(string | undefined), undefined as undefined families=(undefined)]
/// @resolution.place source=route placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=route root=route

    route satisfies Route;
    /// @resolution.name source=route target=route
    /// @resolution.place source=route placement="local" lifetime="static" access="exclusive"
    /// @resolution.access source=route root=route
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
/// @type.symbol symbol=route source=route type=Route reduced=`api:${string}`
/// @resolution.pattern source=route kind=binding target=route
/// @resolution.name source=Route target=Route

if (route == "api:users") {
/// @resolution.name source=route target=route
/// @resolution.operator source="route == \"api:users\"" type=boolean operator="==" kind=builtin operands=[route as `api:${string}` families=(string), "api:users" as "api:users" families=(string)]
/// @resolution.place source=route placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=route root=route

    route satisfies Route;
    /// @resolution.name source=route target=route
    /// @resolution.place source=route placement="local" lifetime="static" access="exclusive"
    /// @resolution.access source=route root=route
    /// @resolution.name source=Route target=Route

} else {
    route satisfies Route;
    /// @resolution.name source=route target=route
    /// @resolution.place source=route placement="local" lifetime="static" access="exclusive"
    /// @resolution.access source=route root=route
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

const route: Route = "api:users" as "api:users" | "api:posts";

const section: "users" | "posts" = match (route) {
    "api:users" => "users"
    "api:posts" => "posts"
};

section satisfies "users" | "posts";

=== checked ===
type Route = `api:${"users" | "posts"}`;
/// @type.symbol symbol=Route source="type Route = `api:${\"users\" | \"posts\"}`" type=`api:${"users" | "posts"}` reduced="api:users" | "api:posts"
/// @definition.type symbol=Route source="type Route = `api:${\"users\" | \"posts\"}`" value=`api:${"users" | "posts"}` reduced="api:users" | "api:posts"

const route: Route = "api:users";
/// @type.symbol symbol=route source=route type=Route reduced="api:users" | "api:posts"
/// @resolution.pattern source=route kind=binding target=route
/// @resolution.name source=Route target=Route

const section = match (route) {
/// @type.symbol symbol=section source=section type="users" | "posts"
/// @resolution.pattern source=section kind=binding target=section
/// @resolution.name source=route target=route
/// @resolution.place source=route placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=route root=route

    "api:users" => "users"
    /// @resolution.pattern source="\"api:users\"" kind=literal value="api:users"

    "api:posts" => "posts"
    /// @resolution.pattern source="\"api:posts\"" kind=literal value="api:posts"

};

section satisfies "users" | "posts";
/// @resolution.name source=section target=section
/// @resolution.place source=section placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=section root=section
"#,
    );
}
