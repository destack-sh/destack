# Materialization

Macro expansion changes visible declarations before checking, and materialization fills checked implementation details after checking.

## bodies

### materialize fills checked bodies

```ds
newtype fastRoute = (string,);

extension of fastRoute implements Patcher<FunctionDeclaration>
{
    static expand(target: FunctionDeclaration, context: ExpansionContext, config: this): void {
        const declaration = comptime eval<Declaration>(ds`
            function ${context.name}(): string;
        `);

        context.replace(declaration);
    }

    static materialize(target: FunctionDeclaration, context: MaterializationContext, config: this): void {
        const value = config[0];
        const implementation = comptime eval<Declaration>(ds`
            function ${context.name}(): string {
                return ${value};
            }
        `);

        context.replace(implementation);
    }
}

@fastRoute("/users")
function route(): string {
    return "/fallback";
}

route satisfies () => string;
```
