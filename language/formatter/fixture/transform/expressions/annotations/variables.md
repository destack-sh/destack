# Variable Annotations

## Variables

### typed variable stays inline

Simple type annotations stay inline in variable declarations.

```tspp
const value: number = 1
```

```tspp expected
const value: number = 1;
```

### decorator prefixed variable type stays inline

Decorator prefixed variable types stay inline after `:`.

```tspp
const buffer: @addrspace("shared") &Buffer = value
```

```tspp expected
const buffer: @addrspace("shared") &Buffer = value;
```

### decorator prefixed variable type stays inline under non-default options

Decorator prefixed variable types stay inline after `:` under non-default formatter options.

```tspp:main.tspp indent-width=2 line-width=80
{
    const buffer: @addrspace("shared") &Buffer = value;
}
```

```tspp expected
{
  const buffer: @addrspace("shared") &Buffer = value;
}
```

### destructuring variable type with trailing marker

Trailing marker comments after typed declarations are preserved.

```tspp:main.tspp
declare const PAGE_PATH: string
  //<- marker
;(()=>{})()
```

```tspp expected
declare const PAGE_PATH: string;
//<- marker
(() => {})();
```

### statement decorator stays on its own line

Statement-level decorators stay above the decorated expression.

```tspp
@trace run()
```

```tspp expected
@trace
run();
```

### statement decorator inside control flow

Statement-level decorators inside blocks stay above the decorated statement.

```tspp
if (ready) { @trace run() } else { @fallback reset() }
```

```tspp expected
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

```tspp
function run(): void { if (ready) { @trace work() } else { try { @fallback recover() } catch (error) { @report handle(error) } } }
```

```tspp expected
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

```tspp
@trace @measure run()
```

```tspp expected
@trace
@measure
run();
```
