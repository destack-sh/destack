# Basic Functions

Tests for function declarations and type checking.

## Function Declarations

### function with no parameters

> Functions can be declared with no parameters.

```ds
function f(): void {}
```

### function with parameters

> Functions can have typed parameters.

```ds
function add(a: number, b: number): void {
    a satisfies number;
    b satisfies number;
}
```

### function with return type

> Functions specify their return type after the parameter list.

```ds
function greet(name: string): string {
    name satisfies string;
    let result = "hello " + name;
    result satisfies string;
    return result;
}
```

## Arrow Functions

### arrow with no parameters

> Arrow functions can be declared with no parameters.

```ds
const f = (): void => {};
f satisfies () => void;
```

### arrow with parameters

> Arrow functions can have typed parameters.

```ds
const add = (a: number, b: number): number => a + b;
add satisfies (a: number, b: number) => number;
```

## Function Assignability

### function value satisfies callable interface

> Function values are assignable to callable interfaces.

```ds
interface Fn {
    (value: string): number
}

const parse = (value: string): number => 1;
parse satisfies Fn;
```

### incompatible return type is not assignable

> Function values with incompatible return types are not assignable.

```ds
interface Fn {
    (value: string): number
}

const parse = (value: string): string => value;
parse satisfies Fn;
```

- contains: expected

### call signature object type accepts function value

> Function values are assignable to call signature object types.

```ds
const fn = (): number => 1;
fn satisfies { (): number };
```

### call signature object type rejects incompatible return type

> Call signature object types reject incompatible return types.

```ds
const fn = (): string => "no";
fn satisfies { (): number };
```

- contains: expected

### callable interface satisfies call signature object type

> Callable interfaces satisfy compatible call signature object types.

```ds
interface Fn {
    (): number
}

const fn: Fn = (): number => 1;
fn satisfies { (): number };
```

## strictFunctionTypes

### strictFunctionTypes rejects narrow parameters

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

### strictFunctionTypes false allows bivariant parameters

```ds:dsconfig.json
{ "compilerOptions": { "strictFunctionTypes": false } }
```

```ds:package.json
{ "name": "spec" }
```

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
