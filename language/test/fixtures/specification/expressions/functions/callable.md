# Callables

Function values satisfy compatible callable shapes.

## assignability

### function values satisfy callable interfaces

> Function values are assignable to callable interfaces.

```ds
interface Fn {
    (value: string): number
}

const parse = (value: string): number => 1;
parse satisfies Fn;
```

### incompatible returns are not assignable

> Function values with incompatible return types are not assignable.

```ds
interface Fn {
    (value: string): number
}

const parse = (value: string): string => value;
parse satisfies Fn;
```

- contains: not assignable

### call signature object types accept function values

> Function values are assignable to call signature object types.

```ds
const fn = (): number => 1;
fn satisfies { (): number };
```

### call signature object types reject incompatible returns

> Call signature object types reject incompatible return types.

```ds
const fn = (): string => "no";
fn satisfies { (): number };
```

- contains: not assignable

### callable interfaces satisfy call signature object types

> Callable interfaces satisfy compatible call signature object types.

```ds
interface Fn {
    (): number
}

const fn: Fn = (): number => 1;
fn satisfies { (): number };
```

## variance

### function assignment rejects narrow parameters

> Function assignment checks parameter variance.

```ds
interface FnWide {
    (value: string | number): void
}

interface FnNarrow {
    (value: string): void
}

function narrow(value: string): void {}
let wide: FnWide = narrow
```

- contains: not assignable
