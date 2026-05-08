# Decorators

Decorator values can implement `Patcher` to change their target during compilation.

## functions

### decorator values select patchers

```ds
newtype route = (string,);

extension of route implements Patcher<FunctionDeclaration>
{
    static expand(target: FunctionDeclaration, context: ExpansionContext, config: this): void {
        const declaration = comptime eval<Declaration>(ds`
            const ${context.name}Path: string = ${config[0]};
        `);

        context.add(declaration);
    }
}

@route("/users")
function users(): string {
    return "users";
}

usersPath satisfies string;
```

### decorator values configure patchers

```ds
newtype exportAs = (string,);

extension of exportAs implements Patcher<FunctionDeclaration>
{
    static expand(target: FunctionDeclaration, context: ExpansionContext, config: this): void {
        context.rename(config[0]);
    }
}

@exportAs("loadUser")
function load(id: string): string {
    return id;
}

loadUser satisfies (id: string) => string;
```

### imported decorators use exported symbols

```ds:macros.ds
export newtype expose = ();

export extension of expose implements Patcher<FunctionDeclaration>
{
    static expand(target: FunctionDeclaration, context: ExpansionContext, config: this): void {
        const declaration = comptime eval<Declaration>(ds`
            const ${context.name}Name: string = "${context.name}";
        `);

        context.add(declaration);
    }
}
```

```ds:main.ds
import { expose } from "./macros.ds";

@expose
function load(id: string): string {
    return id;
}

loadName satisfies string;
```
