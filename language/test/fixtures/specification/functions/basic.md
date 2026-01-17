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

## noImplicitReturns

### noImplicitReturns rejects missing return in block body

> Not all code paths return a value when implicit returns are disabled.

```ds:dsconfig.json
{ "compilerOptions": { "noImplicitReturns": true } }
```

```ds
function example(value: number): number {
    if (value > 0) {
        return value;
    }
}
```

- contains: missing return
- contains: not assignable

### noImplicitReturns allows implicit return expression

> Implicit return expressions satisfy the return requirement.

```ds:dsconfig.json
{ "compilerOptions": { "noImplicitReturns": true } }
```

```ds
function example(value: number): number {
    if (value > 0) {
        return value;
    }
    value + 1
}
```

### noImplicitReturns allows missing returns when false

> Missing return paths are allowed when noImplicitReturns is false.

```ds:dsconfig.json
{ "compilerOptions": { "noImplicitReturns": false } }
```

```ds
function example(value: number): number | void {
    if (value > 0) {
        return value;
    }
}
```

## noImplicitAny

### noImplicitAny rejects implicit parameter types

> Parameters without annotations or defaults are implicit any when strict checking is enabled.

```ds:dsconfig.json
{ "compilerOptions": { "noImplicitAny": true } }
```

```ds
function handle(value) {
}
```

- contains: implicit any type

### noImplicitAny allows defaulted parameters

> Defaults provide an inferred parameter type.

```ds:dsconfig.json
{ "compilerOptions": { "noImplicitAny": true } }
```

```ds
function handle(value = 1) {
    value satisfies number;
}
```

### noImplicitAny rejects uninitialized bindings

> Bindings without annotations or initializers are implicit any.

```ds:dsconfig.json
{ "compilerOptions": { "noImplicitAny": true } }
```

```ds
let pending;
```

- contains: implicit any type

## noImplicitThis

### noImplicitThis rejects implicit this in functions

> `this` inside functions requires an explicit `this` parameter in strict mode.

```ds:dsconfig.json
{ "compilerOptions": { "noImplicitThis": true } }
```

```ds
function counter() {
    this;
}
```

- contains: implicit this type

### noImplicitThis allows explicit this parameters

> Explicit `this` parameters provide a concrete type.

```ds:dsconfig.json
{ "compilerOptions": { "noImplicitThis": true } }
```

```ds
function counter(this: { value: number }) {
    this.value satisfies number;
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

## Function Members

### function toString resolves

> Function values expose Function prototype members.

```ds:dsconfig.json
{ "compilerOptions": { "noAny": false } }
```

```ds libs=es5
const fn = (value: number): number => value + 1;
const text = fn.toString();
text satisfies string;
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

## Declaration Files

### declaration file overloads merge

> Overloads in declaration files merge into a single callable.

```ds:types.d.ds
export function apply(value: string): number;
export function apply(value: number): string;
```

```ds:main.ds
import { apply } from "./types.d.ds";

apply("ok") satisfies number;
apply(42) satisfies string;
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
