# Decorators

Decorator values can implement `Patcher` to change their target during compilation.

## functions

### route decorator adds a route definition

```ds
newtype route = (string,);

extension of route implements Patcher<FunctionDeclaration>
{
    static expand(target: FunctionDeclaration, context: ExpansionContext, config: this): void {
        context.ensureDeclaration("RouteDefinition", () => comptime eval<Declaration>(ds`
            type RouteDefinition = {
                path: string;
                handler: unknown;
            };
        `));

        const declaration = comptime eval<Declaration>(ds`
            const ROUTE_DEFINITION_${context.name.toUpperCase()} = {
                path: "${config[0]}",
                handler: ${context.name},
            } as const satisfies RouteDefinition;
        `);

        context.add(declaration);
    }
}

@route("/users")
function users(): string {
    return "users";
}

ROUTE_DEFINITION_USERS.path satisfies "/users";
ROUTE_DEFINITION_USERS.handler satisfies () => string;
```

### imported decorators expand through exported symbols

```ds:macros.ds
export newtype route = (string,);

export extension of route implements Patcher<FunctionDeclaration>
{
    static expand(target: FunctionDeclaration, context: ExpansionContext, config: this): void {
        context.ensureDeclaration("RouteDefinition", () => comptime eval<Declaration>(ds`
            type RouteDefinition = {
                path: string;
                handler: unknown;
            };
        `));

        const declaration = comptime eval<Declaration>(ds`
            const ROUTE_DEFINITION_${context.name.toUpperCase()} = {
                path: "${config[0]}",
                handler: ${context.name},
            } as const satisfies RouteDefinition;
        `);

        context.add(declaration);
    }
}
```

```ds:main.ds
import { route } from "./macros.ds";

@route("/load")
function load(id: string): string {
    return id;
}

ROUTE_DEFINITION_LOAD.path satisfies "/load";
ROUTE_DEFINITION_LOAD.handler satisfies (id: string) => string;
```
