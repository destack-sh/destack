# Patch

Patchers edit the visible declaration set with add, replace, rename, and remove.

## wrap

### rename and add wrap a function

Function decorators can wrap a target by renaming the original and adding a new declaration.

```ds
newtype logged = ();

declare function log(message: string): void;

extension<F> of logged implements Patcher<F>
    where F extends (...args: unknown[]) => unknown
{
    static patch(target: F, context: PatchContext, config: this): Patch[] {
        const innerName = `${context.name}Inner`;
        const wrapper = comptime eval<Declaration>(ds`
            function ${context.name}(id: string): string {
                log("load");
                return ${innerName}(id);
            }
        `);

        return [
            Patch.rename({ symbol: context.symbol, name: innerName }),
            Patch.add({ scope: context.scope, declaration: wrapper }),
        ];
    }
}

@logged
function load(id: string): string {
    return id;
}

load satisfies (id: string) => string;
```

### configuration is ordinary data

Patch providers receive the decorator value as configuration.

```ds
newtype memoize = {
    capacity?: uint;
};

declare function readCachedUser(id: string): string | undefined;
declare function writeCachedUser(id: string, value: string, capacity: uint): void;

extension<F> of memoize implements Patcher<F>
    where F extends (...args: unknown[]) => unknown
{
    static patch(target: F, context: PatchContext, config: this): Patch[] {
        const capacity = config.capacity ?? 256;
        const innerName = `${context.name}Inner`;
        const wrapper = comptime eval<Declaration>(ds`
            function ${context.name}(id: string): string {
                const cached = readCachedUser(id);
                if (cached != undefined) {
                    return cached;
                }

                const value = ${innerName}(id);
                writeCachedUser(id, value, ${capacity});
                return value;
            }
        `);

        return [
            Patch.rename({ symbol: context.symbol, name: innerName }),
            Patch.add({ scope: context.scope, declaration: wrapper }),
        ];
    }
}

@memoize({ capacity: 1024 })
function load(id: string): string {
    return id;
}

load satisfies (id: string) => string;
```

## operations

### add creates a new declaration

Add patches create visible declarations.

```ds
newtype exposeMetrics = ();

extension<T> of exposeMetrics implements Patcher<T> {
    static patch(target: T, context: PatchContext, config: this): Patch[] {
        const declaration = comptime eval<Declaration>(ds`
            function getRequestCount(): uint64 {
                return 0;
            }
        `);

        return [Patch.add({ scope: context.scope, declaration })];
    }
}

@exposeMetrics
class Server {
    start(): void {}
}

getRequestCount satisfies () => uint64;
```

### replace redirects a symbol

Replace patches redirect symbols to generated declarations.

```ds
newtype constantRoute = (string,);

extension<F> of constantRoute implements Patcher<F>
    where F extends (...args: unknown[]) => unknown
{
    static patch(target: F, context: PatchContext, config: this): Patch[] {
        const value = config[0];
        const replacement = comptime eval<Declaration>(ds`
            function ${context.name}(): string {
                return ${value};
            }
        `);

        return [Patch.replace({ symbol: context.symbol, declaration: replacement })];
    }
}

@constantRoute("/users")
function route(): string {
    return "/fallback";
}

route satisfies () => string;
```

### rename changes the visible name

Rename patches change visible names without adding declarations.

```ds
newtype publicName = (string,);

extension<F> of publicName implements Patcher<F>
    where F extends (...args: unknown[]) => unknown
{
    static patch(target: F, context: PatchContext, config: this): Patch[] {
        return [Patch.rename({ symbol: context.symbol, name: config[0] })];
    }
}

@publicName("loadUser")
function load(id: string): string {
    return id;
}

loadUser satisfies (id: string) => string;
```

## limits

### patchers are not patched

Declarations that implement `Patcher` are analyzed before patch expansion.

```ds
@derive(Clone)
newtype logged = ();

extension<F> of logged implements Patcher<F>
    where F extends (...args: unknown[]) => unknown
{
    static patch(target: F, context: PatchContext, config: this): Patch[] {
        return [];
    }
}
```

- contains: Patcher

### patches use declarations

Patch declarations are `Declaration` handles, not arbitrary values.

```ds
newtype bad = ();

extension<F> of bad implements Patcher<F>
    where F extends (...args: unknown[]) => unknown
{
    static patch(target: F, context: PatchContext, config: this): Patch[] {
        return [Patch.replace({ symbol: context.symbol, declaration: "bad" })];
    }
}

@bad
function load(id: string): string {
    return id;
}
```

- contains: Declaration
