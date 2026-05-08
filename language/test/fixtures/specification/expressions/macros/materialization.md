# Materialization

Expansion fixes the visible declaration graph, and materialization fills checked implementation details.

## bodies

### materialize receives expansion state

```ds
newtype fastRoute = (string,);

type FastRouteState = {
    value: string;
};

extension of fastRoute implements Patcher<FunctionDeclaration, FastRouteState>
{
    static expand(
        target: FunctionDeclaration,
        context: ExpansionContext,
        config: this,
    ): FastRouteState {
        const declaration = comptime eval<Declaration>(ds`
            function ${context.name}(): string;
        `);

        context.replace(declaration);

        return {
            value: config[0],
        };
    }

    static materialize(
        target: FunctionDeclaration,
        context: MaterializationContext,
        config: this,
        state: FastRouteState,
    ): void {
        const implementation = comptime eval<Declaration>(ds`
            function ${context.name}(): string {
                return ${state.value};
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

### materialize cannot add visible declarations

```ds
newtype lateExport = ();

extension of lateExport implements Patcher<FunctionDeclaration>
{
    static materialize(
        target: FunctionDeclaration,
        context: MaterializationContext,
        config: this,
        state: void,
    ): void {
        const declaration = comptime eval<Declaration>(ds`
            function lateName(): string;
        `);

        context.add(declaration);
    }
}

@lateExport
function load(): string {
    return "load";
}
```

- contains: add
