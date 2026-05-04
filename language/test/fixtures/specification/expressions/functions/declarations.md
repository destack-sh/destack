# Functions

Function declarations and type checking.

## function declarations

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

## rejections

### declare functions cannot have bodies

> Declared functions cannot include bodies.

```ds
declare function greet(): string {
    return "hi";
}
```

- contains: invalid function

### arrow functions cannot declare explicit this parameters

> Value level arrow functions cannot declare explicit this parameters.

```ds
let f = (this: string) => {}
```

- contains: invalid function

## returns

### block bodies reject missing returns

> Not all code paths return a value.

```ds
function example(value: number): number {
    if (value > 0) {
        return value;
    }
}
```

- contains: missing return
- contains: not assignable

### tail expressions satisfy return types

> Implicit return expressions satisfy the return requirement.

```ds
function example(value: number): number {
    if (value > 0) {
        return value;
    }
    value + 1
}
```

## implicit types

### parameters require types

> Parameters without annotations or defaults are implicit any and are rejected.

```ds
function handle(value) {
}
```

- contains: implicit any type

### parameter defaults infer parameter types

> Defaults provide an inferred parameter type.

```ds
function handle(value = 1) {
    value satisfies number;
}
```

### uninitialized bindings require annotations

> Bindings without annotations or initializers are implicit any.

```ds
let pending;
```

- contains: implicit any type

## this

### functions reject implicit this

> `this` inside functions requires an explicit `this` parameter.

```ds
function counter() {
    this;
}
```

- contains: implicit this type

### explicit this parameters provide receiver types

> Explicit `this` parameters provide a concrete type.

```ds
function counter(this: { value: number }) {
    this.value satisfies number;
}
```

### methods provide implicit this

> Member methods have an implicit `this` binding.

```ds
class Counter {
    value: number = 0;

    add(value: number): number {
        return this.value + value;
    }
}
```

### method lambdas capture this

> Lambdas inside methods capture the lexical `this`.

```ds
class Counter {
    value: number = 0;

    make(): () => number {
        return () => this.value;
    }
}
```

### non-member lambdas reject implicit this

> Lambdas outside methods require an explicit `this` parameter to use `this`.

```ds
function make() {
    return () => this;
}
```

- contains: implicit this type

## arrow functions

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

## function assignability

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

- contains: not assignable

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

- contains: not assignable

### callable interface satisfies call signature object type

> Callable interfaces satisfy compatible call signature object types.

```ds
interface Fn {
    (): number
}

const fn: Fn = (): number => 1;
fn satisfies { (): number };
```

## declaration files

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

## parameter variance

### function assignment rejects narrow parameters

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
