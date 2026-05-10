# Expansion

Expansion edits the visible declaration graph before checking.

## declarations

### add creates a sibling declaration

```ds
newtype exposeMetrics = ();

extension of exposeMetrics implements Macro<ClassDeclaration> {
    static expand(target: ClassDeclaration, context: ExpansionContext, config: this): void {
        const declaration = comptime eval<Declaration>(ds`
            const ${context.name}Metrics = {
                requests: 0uint64,
            } as const;
        `);

        context.add(declaration);
    }
}

@exposeMetrics
class Server {
    start(): void {}
}

ServerMetrics.requests satisfies uint64;
```

### addChild creates a member

```ds
newtype observable = ();

type ChangeEvent = {
    field: string;
};

interface Disposable {
    dispose(): void;
}

extension of observable implements Macro<ClassDeclaration> {
    static expand(target: ClassDeclaration, context: ExpansionContext, config: this): void {
        const method = comptime eval<Member>(ds`
            onChange(listener: (event: ChangeEvent) => void): Disposable;
        `);

        context.addChild(method);
    }
}

@observable
class Store {
    value: int32;
}

const store = new Store();
store.onChange satisfies (listener: (event: ChangeEvent) => void) => Disposable;
```

### generated decorators expand in the same fixed point

```ds
newtype route = (string,);
newtype exposeHealth = ();

extension of route implements Macro<FunctionDeclaration> {
    static expand(target: FunctionDeclaration, context: ExpansionContext, config: this): void {
        context.ensureDeclaration(
            "RouteDefinition",
            () =>
                comptime eval<Declaration>(ds`
            type RouteDefinition = {
                path: string;
                handler: unknown;
            };
        `),
        );

        const declaration = comptime eval<Declaration>(ds`
            const ROUTE_DEFINITION_${context.name.toUpperCase()} = {
                path: "${config[0]}",
                handler: ${context.name},
            } as const satisfies RouteDefinition;
        `);

        context.add(declaration);
    }
}

extension of exposeHealth implements Macro<ClassDeclaration> {
    static expand(target: ClassDeclaration, context: ExpansionContext, config: this): void {
        const declaration = comptime eval<Declaration>(ds`
            @route("/health")
            function health(): string {
                return "ok";
            }
        `);

        context.add(declaration);
    }
}

@exposeHealth
class Server {}

ROUTE_DEFINITION_HEALTH.path satisfies "/health";
ROUTE_DEFINITION_HEALTH.handler satisfies () => string;
```
