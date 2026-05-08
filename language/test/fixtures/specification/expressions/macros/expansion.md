# Expansion

Expansion edits the visible declaration graph before checking.

## declarations

### add creates a sibling declaration

```ds
newtype exposeMetrics = ();

extension of exposeMetrics implements Patcher<ClassDeclaration> {
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

extension of observable implements Patcher<ClassDeclaration> {
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
