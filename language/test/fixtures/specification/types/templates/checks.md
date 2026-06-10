# Template Checks

Template types check their span types.

## checks

### nullish checks preserve template literal constraints

Nullish checks preserve template constraints in the non-null branch.
The narrowed branch keeps the original template type.

```ds
type Route = `api:${string}`;

declare const route: Route | undefined;

if (route != undefined) {
    route satisfies Route;
}
```

### equality checks keep template literal constraints in both branches

Equality checks keep template constraints in both branches.
Branch narrowing does not widen away the template-literal constraint.

```ds
type Route = `api:${string}`;

declare const route: Route;

if (route == "api:users") {
    route satisfies Route;
} else {
    route satisfies Route;
}
```

### match over template literal unions can be exhaustive without fallback

Match over template-literal unions can be exhaustive without fallback.
Exhaustive literal arms type the result as the arm-result union.

```ds
type Route = "api:users" | "api:posts";

const route: Route = "api:users";

const section = match (route) {
    "api:users" => "users"
    "api:posts" => "posts"
};

section satisfies "users" | "posts";
```

### match let bindings widen template literal result literals

`let` bindings widen match result literals after control-flow joins.
Template-literal-originated string outputs still follow ordinary widening.

```ds
type Route = "api:users" | "api:posts";

const route: Route = "api:users";

let section = match (route) {
    "api:users" => "users"
    "api:posts" => "posts"
};

section satisfies string;
```

### match const bindings keep template literal result literal unions

`const` bindings keep match result literal unions.
Const and let bindings keep distinct widening rules.

```ds
type Route = "api:users" | "api:posts";

const route: Route = "api:users";

const section = match (route) {
    "api:users" => "users"
    "api:posts" => "posts"
};

section satisfies "users" | "posts";
```
