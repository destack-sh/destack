# Decorators

Decorator values can implement `Patcher` to rewrite their target.

## functions

### rename and add wrap a function

```ds
newtype logged = ();

declare function log(message: string): void;

extension of logged implements Patcher<FunctionDeclaration>
{
    static expand(target: FunctionDeclaration, context: ExpansionContext, config: this): void {
        const innerName = `${context.name}Inner`;
        const wrapper = comptime eval<Declaration>(ds`
            function ${context.name}(id: string): string {
                log(`${context.name} id=${id}`);
                return ${innerName}(id);
            }
        `);

        context.rename(innerName);
        context.add(wrapper);
    }
}

@logged
function load(id: string): string {
    return id;
}

load satisfies (id: string) => string;
```

### decorator values configure patchers

```ds
newtype memoize = {
    capacity?: uint;
};

declare function readCachedUser(id: string): string | undefined;
declare function writeCachedUser(id: string, value: string, capacity: uint): void;

extension of memoize implements Patcher<FunctionDeclaration>
{
    static expand(target: FunctionDeclaration, context: ExpansionContext, config: this): void {
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

        context.rename(innerName);
        context.add(wrapper);
    }
}

@memoize({ capacity: 1024 })
function load(id: string): string {
    return id;
}

load satisfies (id: string) => string;
```
