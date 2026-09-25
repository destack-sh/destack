use crate::tests::{DirRows, TestSession};

/// A nullish guard keeps the template literal type of the narrowed value.
#[test]
fn test_preserve_a_template_literal_constraint_through_a_nullish_guard() {
    let session = TestSession::single(
        r#"
type Route = `api:${string}`;

declare const route: Route | undefined;

if (route != undefined) {
    route satisfies Route;
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Route = `api:${string}`;

declare const route: `api:${string}` | undefined;

if (route != undefined) {
    route satisfies Route;
}

=== dir ===
type Route = `api:${string}`;
/// @type.symbol symbol=Route source="type Route = `api:${string}`" type=`api:${string}`
/// @definition.type symbol=Route source="type Route = `api:${string}`" value=`api:${string}`

declare const route: Route | undefined;
/// @type.symbol symbol=route source=route type=`api:${string}` | undefined
/// @resolution.pattern source=route kind=binding target=route
/// @resolution.name source=Route target=Route

if (route != undefined) {
/// @resolution.name source=route target=route
/// @resolution.operator source="route != undefined" type=boolean operator="!=" kind=builtin operands=[route as `api:${string}` | undefined families=(string | undefined), undefined as undefined families=(undefined)]
/// @resolution.place source=route placement="local" lifetime="static" access="immutable"
/// @resolution.access source=route root=route

    route satisfies Route;
    /// @resolution.name source=route target=route
    /// @resolution.place source=route placement="local" lifetime="static" access="immutable"
    /// @resolution.access source=route root=route
    /// @resolution.narrowing source=route union=`api:${string}` | undefined arms=`api:${string}`
    /// @resolution.name source=Route target=Route

}
"#,
    );
}

/// An equality guard keeps the template literal type on both branches.
#[test]
fn test_keep_a_template_literal_constraint_through_an_equality_guard() {
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

    session.assert_dir(
        "main.tspp",
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

=== dir ===
type Route = `api:${string}`;
/// @type.symbol symbol=Route source="type Route = `api:${string}`" type=`api:${string}`
/// @definition.type symbol=Route source="type Route = `api:${string}`" value=`api:${string}`

declare const route: Route;
/// @type.symbol symbol=route source=route type=Route
/// @resolution.pattern source=route kind=binding target=route
/// @resolution.name source=Route target=Route

if (route == "api:users") {
/// @resolution.name source=route target=route
/// @resolution.operator source="route == \"api:users\"" type=boolean operator="==" kind=builtin operands=[route as `api:${string}` families=(string), "api:users" as "api:users" families=(string)]
/// @resolution.place source=route placement="local" lifetime="static" access="immutable"
/// @resolution.access source=route root=route

    route satisfies Route;
    /// @resolution.name source=route target=route
    /// @resolution.place source=route placement="local" lifetime="static" access="immutable"
    /// @resolution.access source=route root=route
    /// @resolution.name source=Route target=Route

} else {
    route satisfies Route;
    /// @resolution.name source=route target=route
    /// @resolution.place source=route placement="local" lifetime="static" access="immutable"
    /// @resolution.access source=route root=route
    /// @resolution.name source=Route target=Route

}
"#,
    );
}

/// A match over a template of a literal union covers every value it admits.
#[test]
fn test_treat_a_match_over_a_template_literal_type_union_as_exhaustive() {
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

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Route = `api:${"users" | "posts"}`;

const route: Route = "api:users";

const section: "users" | "posts" = match (route) {
    "api:users" => "users"
    "api:posts" => "posts"
};

section satisfies "users" | "posts";

=== dir ===
type Route = `api:${"users" | "posts"}`;
/// @type.symbol symbol=Route source="type Route = `api:${\"users\" | \"posts\"}`" type="api:users" | "api:posts"
/// @definition.type symbol=Route source="type Route = `api:${\"users\" | \"posts\"}`" value=`api:${"users" | "posts"}`

const route: Route = "api:users";
/// @type.symbol symbol=route source=route type=Route
/// @resolution.pattern source=route kind=binding target=route
/// @resolution.name source=Route target=Route

const section = match (route) {
/// @type.symbol symbol=section source=section type="users" | "posts"
/// @resolution.pattern source=section kind=binding target=section
/// @resolution.coverage exhaustive=true disjoint=true
/// @resolution.name source=route target=route
/// @resolution.place source=route placement="local" lifetime="static" access="immutable"
/// @resolution.access source=route root=route

    "api:users" => "users"
    /// @resolution.pattern source="\"api:users\"" kind=literal value="api:users"

    "api:posts" => "posts"
    /// @resolution.pattern source="\"api:posts\"" kind=literal value="api:posts"

};

section satisfies "users" | "posts";
/// @resolution.name source=section target=section
/// @resolution.place source=section placement="local" lifetime="static" access="immutable"
/// @resolution.access source=section root=section
"#,
    );
}
