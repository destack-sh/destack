# Variable Annotations

## Variables

### typed variable stays inline

Simple type annotations stay inline in variable declarations.

```ds
const value: number = 1
```

```ds expected
const value: number = 1;
```

### decorator prefixed variable type stays inline

Decorator prefixed variable types stay inline after `:`.

```ds
const buffer: @addrspace("shared") &Buffer = value
```

```ds expected
const buffer: @addrspace("shared") &Buffer = value;
```

### TypeScript decorator prefixed variable type stays inline

Decorator prefixed variable types stay inline after `:` under non-default formatter options.

```ts:main.ts indent-width=2 line-width=80
{
    const buffer: @addrspace("shared") &Buffer = value;
}
```

```ts expected
{
  const buffer: @addrspace("shared") &Buffer = value;
}
```

### destructuring variable type with trailing marker

Trailing marker comments after typed declarations are preserved.

```ts:main.ts
declare const PAGE_PATH: string
  //<- marker
;(()=>{})()
```

```ts expected
declare const PAGE_PATH: string;
    //<- marker
(() => {})();
```

### statement decorator stays on its own line

Statement-level decorators stay above the decorated expression.

```ds
@trace run()
```

```ds expected
@trace
run();
```

### statement decorator inside control flow

Statement-level decorators inside blocks stay above the decorated statement.

```ds
if (ready) { @trace run() } else { @fallback reset() }
```

```ds expected
if (ready) {
    @trace
    run()
} else {
    @fallback
    reset()
}
```

### statement decorator inside nested control flow

Statement decorators stay above their statement inside nested value branches.

```ds
function run(): void { if (ready) { @trace work() } else { try { @fallback recover() } catch (error) { @report handle(error) } } }
```

```ds expected
function run(): void {
    if (ready) {
        @trace
        work()
    } else {
        try {
            @fallback
            recover()
        } catch (error) {
            @report
            handle(error)
        }
    }
}
```

### stacked statement decorators

Stacked statement decorators each stay on their own line.

```ds
@trace @measure run()
```

```ds expected
@trace
@measure
run();
```
