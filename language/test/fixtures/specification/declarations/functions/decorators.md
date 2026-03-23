# Function Decorators

Tests for function-level decorator markers and validation.

## Intrinsic decorator markers

### mustUse decorator allows call sites

> mustUse marks functions without producing errors.

```ds
@mustUse
function openFile(): int {
    return 1;
}

openFile();
```

### pure decorator allows declarations

> pure marks functions without producing errors.

```ds
@pure
function add(a: int, b: int): int {
    return a + b;
}
```

### tailcall decorator allows declarations

> tailcall marks functions without producing errors.

```ds
@tailcall
function loop(n: int): int {
    if (n <= 0) {
        return 0;
    }
    return loop(n - 1);
}
```

### unsafe decorator allows declarations

> unsafe marks functions without producing errors.

```ds
@unsafe
function readPtr(ptr: *u8): u8 {
    return *ptr;
}
```

### transmute decorator allows declarations

> transmute marks functions without producing errors.

```ds
@transmute
function bits(value: int): int {
    return value;
}
```

## Decorator expressions

### decorator expressions accept call chains

> decorator expressions accept call chains and member access.

```ds
function factory(): { decorator: () => void } {
    return { decorator: () => { } }
}

function decorator(): { member: () => void } {
    return { member: () => { } }
}

@factory().decorator()
@factory().decorator
@decorator().member()
@decorator().member
function demo(): void { }
```

### function decorators are accepted

> Decorators can resolve to callable functions.

```ds
function deco(target: unknown): void {}

@deco
function demo(): void { }
```

### decorator expressions accept static arguments

> decorator expressions accept static arguments without a call.

```ds
function decorator<T>(): void { }

@decorator<int>
function demo(): void { }
```

### hot decorator rejects arguments

> hot does not accept arguments.

```ds
@hot(1)
function parse(): void { }
```

- contains: hot decorator does not accept arguments

### cold decorator rejects arguments

> cold does not accept arguments.

```ds
@cold("log")
function logError(): void { }
```

- contains: cold decorator does not accept arguments

### likely decorator rejects arguments

> likely does not accept arguments.

```ds
@likely(true)
function handle(): void { }
```

- contains: likely decorator does not accept arguments

### unlikely decorator rejects arguments

> unlikely does not accept arguments.

```ds
@unlikely(true)
function handle(): void { }
```

- contains: unlikely decorator does not accept arguments

### hot and cold decorators conflict

> hot and cold cannot be combined.

```ds
@hot
@cold
function parse(): void { }
```

- contains: hot and cold decorators cannot be combined

### likely and unlikely decorators conflict

> likely and unlikely cannot be combined.

```ds
@likely
@unlikely
function dispatch(): void { }
```

- contains: likely and unlikely decorators cannot be combined

## Additional targets

### parameter decorators are accepted

> Decorators can be applied to function parameters.

```ds
function greet(@deprecated name: string): string {
    return name;
}

greet("hi") satisfies string;
```

### statement decorators are accepted

> Decorators can be applied to statements like loops.

```ds
@unroll
for (let i = 0; i < 4; i++) {
    let _ = i;
}
```
