# Expansion

Macro expansion edits a module with `add`, `addChild`, `replace`, `rename`, and `remove`.

## declarations

### add creates a declaration

```ds
newtype exposeMetrics = ();

extension of exposeMetrics implements Patcher<ClassDeclaration> {
    static expand(target: ClassDeclaration, context: ExpansionContext, config: this): void {
        const declaration = comptime eval<Declaration>(ds`
            function getRequestCount(): uint64 {
                return 0;
            }
        `);

        context.add(declaration);
    }
}

@exposeMetrics
class Server {
    start(): void {}
}

getRequestCount satisfies () => uint64;
```

### addChild adds a declaration child

```ds
newtype observable = ();

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

### replace redirects a symbol

```ds
newtype constantRoute = (string,);

extension of constantRoute implements Patcher<FunctionDeclaration>
{
    static expand(target: FunctionDeclaration, context: ExpansionContext, config: this): void {
        const value = config[0];
        const replacement = comptime eval<Declaration>(ds`
            function ${context.name}(): string {
                return ${value};
            }
        `);

        context.replace(replacement);
    }
}

@constantRoute("/users")
function route(): string {
    return "/fallback";
}

route satisfies () => string;
```

### rename changes the visible name

```ds
newtype publicName = (string,);

extension of publicName implements Patcher<FunctionDeclaration>
{
    static expand(target: FunctionDeclaration, context: ExpansionContext, config: this): void {
        context.rename(config[0]);
    }
}

@publicName("loadUser")
function load(id: string): string {
    return id;
}

loadUser satisfies (id: string) => string;
```

### remove hides a symbol

```ds
newtype internal = ();

extension of internal implements Patcher<FunctionDeclaration>
{
    static expand(target: FunctionDeclaration, context: ExpansionContext, config: this): void {
        context.remove();
    }
}

@internal
function helper(): string {
    return "hidden";
}

helper;
```

- contains: helper

### replacement declarations are typed

```ds
newtype bad = ();

extension of bad implements Patcher<FunctionDeclaration>
{
    static expand(target: FunctionDeclaration, context: ExpansionContext, config: this): void {
        context.replace("bad");
    }
}

@bad
function load(id: string): string {
    return id;
}
```

- contains: Declaration
