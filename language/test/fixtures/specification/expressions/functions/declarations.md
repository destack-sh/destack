# Functions

Function declarations and type checking.

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

### declare async functions are allowed

> Declared function signatures can use async markers.

```ds
declare async function load(): void
```

### declare generator functions are allowed

> Declared function signatures can use generator markers.

```ds
declare function* ids(): void
```

## invalid declarations

### declare functions cannot have bodies

> Declared functions cannot include bodies.

```ds
declare function greet(): string {
    return "hi";
}
```

- invalid function

### arrow functions cannot declare explicit this parameters

> Value level arrow functions cannot declare explicit this parameters.

```ds
let f = (this: string) => {}
```

- invalid function

## noImplicitReturns

### noImplicitReturns rejects missing return in block body

> Not all code paths return a value when implicit returns are disabled.

```json:destack.json
{ "compiler": { "noImplicitReturns": true } }
```

```ds
function example(value: number): number {
    if (value > 0) {
        return value;
    }
}
```

- missing return
- contains: not assignable

### noImplicitReturns allows implicit return expression

> Implicit return expressions satisfy the return requirement.

```json:destack.json
{ "compiler": { "noImplicitReturns": true } }
```

```ds
function example(value: number): number {
    if (value > 0) {
        return value;
    }
    value + 1
}
```

## noImplicitAny

### noImplicitAny rejects implicit parameter types

> Parameters without annotations or defaults are implicit any when strict checking is enabled.

```json:destack.json
{ "compiler": { "noImplicitAny": true } }
```

```ds
function handle(value) {
}
```

- implicit any type

### noImplicitAny allows defaulted parameters

> Defaults provide an inferred parameter type.

```json:destack.json
{ "compiler": { "noImplicitAny": true } }
```

```ds
function handle(value = 1) {
    value satisfies number;
}
```

### noImplicitAny rejects uninitialized bindings

> Bindings without annotations or initializers are implicit any.

```json:destack.json
{ "compiler": { "noImplicitAny": true } }
```

```ds
let pending;
```

- implicit any type

## noImplicitThis

### noImplicitThis rejects implicit this in functions

> `this` inside functions requires an explicit `this` parameter in strict mode.

```json:destack.json
{ "compiler": { "noImplicitThis": true } }
```

```ds
function counter() {
    this;
}
```

- implicit this type

### noImplicitThis allows explicit this parameters

> Explicit `this` parameters provide a concrete type.

```json:destack.json
{ "compiler": { "noImplicitThis": true } }
```

```ds
function counter(this: { value: number }) {
    this.value satisfies number;
}
```

### noImplicitThis allows implicit this in methods

> Member methods have an implicit `this` binding even in strict mode.

```json:destack.json
{ "compiler": { "noImplicitThis": true } }
```

```ds
class Counter {
    value: number = 0;

    add(value: number): number {
        return this.value + value;
    }
}
```

### noImplicitThis allows implicit this in method lambdas

> Lambdas inside methods capture the lexical `this`.

```json:destack.json
{ "compiler": { "noImplicitThis": true } }
```

```ds
class Counter {
    value: number = 0;

    make(): () => number {
        return () => this.value;
    }
}
```

### noImplicitThis rejects implicit this in non-member lambdas

> Lambdas outside methods require an explicit `this` parameter to use `this`.

```json:destack.json
{ "compiler": { "noImplicitThis": true } }
```

```ds
function make() {
    return () => this;
}
```

- implicit this type

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

