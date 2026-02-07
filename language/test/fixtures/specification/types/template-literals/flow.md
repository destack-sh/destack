# Template Literal Flow

## flow

### nullish checks preserve template literal constraints

> Nullish checks should preserve template-literal constraints in the non-null branch.
> The narrowed branch should keep the original template-literal type.

```ds
type Route = `api:${string}`;

declare const route: Route | undefined;

if route != undefined {
    route satisfies Route;
}
```

### equality checks keep template literal constraints in both branches

> Equality checks should keep template-literal constraints in both branches.
> This verifies branch flow does not widen away the template-literal constraint.

```ds
type Route = `api:${string}`;

declare const route: Route;

if route == "api:users" {
    route satisfies Route;
} else {
    route satisfies Route;
}
```

### match over template literal unions can be exhaustive without fallback

> Match over template-literal unions can be exhaustive without fallback.
> Exhaustive literal arms should type the result as the arm-result union.

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

> `let` bindings should widen match result literals after control-flow joins.
> This locks widening behavior for template-literal-originated string outputs.

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

> `const` bindings should keep match result literal unions.
> This ensures commit and widening rules stay distinct for const vs let.

```ds
type Route = "api:users" | "api:posts";

const route: Route = "api:users";

const section = match (route) {
    "api:users" => "users"
    "api:posts" => "posts"
};

section satisfies "users" | "posts";
```
