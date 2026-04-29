# Function Declarations

Tests for function declaration formatting.

## Basic Functions

### simple function

Extra whitespace in the function signature should be removed.

```ds
function   foo  (  )   {   }
```

Empty function bodies stay on one line with a space inside the braces.

```ds expected
function foo() {}
```

### function with parameters

Parameter spacing should be normalized with no space after `(` or before `)`.

```ds
function   foo  (  x  :  number  ,  y  :  string  )   {   }
```

```ds expected
function foo(x: number, y: string) {}
```

### function with return type

Functions with statements in the body get their body broken to multiple lines.

```ds
function   foo  (  )  :  number   {  return 1  }
```

```ds expected
function foo(): number {
    return 1;
}
```

### function with body

Return statements cause the function body to expand to multiple lines.

```ds
function add(a: number, b: number): number { return a + b }
```

```ds expected
function add(a: number, b: number): number {
    return a + b;
}
```

### function with multiple statements

Each statement goes on its own line with proper indentation.

```ds
function process(x: number) { const y = x * 2; const z = y + 1; return z; }
```

```ds expected
function process(x: number) {
    const y = x * 2;
    const z = y + 1;
    return z;
}
```

## Async Functions

### async function

The `async` keyword precedes `function` with a single space between them.

```ds
async   function   foo  (  )   {   }
```

```ds expected
async function foo() {}
```

### async function with await

Await expressions are preserved inside async function bodies.

```ds
async function fetch(url: string) { const res = await request(url); return res }
```

```ds expected
async function fetch(url: string) {
    const res = await request(url);
    return res;
}
```

### async function with return type

Async functions typically return Promise types.

```ds
async function getData(): Promise<Data> { return await fetchData() }
```

```ds expected
async function getData(): Promise<Data> {
    return await fetchData();
}
```

## Generator Functions

### generator function

The `*` attaches to the `function` keyword with no space.

```ds
function  *  foo  (  )   {   }
```

```ds expected
function* foo() {}
```

### generator with yield

Single-statement loops stay on one line.

```ds
function* range(start: number, end: number) { for (let i = start; i < end; i++) { yield i } }
```

```ds expected
function* range(start: number, end: number) {
    for (let i = start; i < end; i++) {
        yield i;
    }
}
```

### async generator

String literals stay normalized inside async generators.

```ds
async function* items() { yield await fetch("a"); yield await fetch("b") }
```

```ds expected
async function* items() {
    yield await fetch("a");
    yield await fetch("b");
}
```

## Arrow Functions

### arrow function expression

Arrow functions with expression bodies stay on one line. Single parameters get parentheses.

```ds
const   foo   =   (  x  )   =>   x  +  1
```

```ds expected
const foo = (x) => x + 1;
```

### arrow function with body

Arrow functions with block bodies get broken to multiple lines.

```ds
const   foo   =   (  x  )   =>   {   return  x  +  1   }
```

```ds expected
const foo = (x) => {
    return x + 1;
};
```

### arrow function with type

Type annotations on arrow function variables are preserved.

```ds
const foo: (x: number) => number = (x) => x + 1
```

```ds expected
const foo: (x: number) => number = (x) => x + 1;
```

### module ts arrow generic keeps trailing comma in mts

Single generic arrow type parameters in `.mts` keep a trailing comma.

```ts:main.mts
const fn = <T,>() => {}
```

```ts expected
const fn = <T,>() => {};
```

### module ts arrow generic keeps trailing comma in cts

Single generic arrow type parameters in `.cts` keep a trailing comma.

```ts:main.cts
const fn = <T,>() => {}
```

```ts expected
const fn = <T,>() => {};
```

## Generic Functions

### generic function

Type parameters appear in angle brackets after the function name.

```ds
function identity<T>(x: T): T { return x }
```

```ds expected
function identity<T>(x: T): T {
    return x;
}
```

### generic with constraint

Type constraints use colon syntax: `T: Constraint`.

```ds
function process<T: Comparable>(a: T, b: T): boolean { return a < b }
```

```ds expected
function process<T: Comparable>(a: T, b: T): boolean {
    return a < b;
}
```

### multiple type parameters

Multiple type parameters are separated by commas with no trailing comma.

```ds
function merge<T, U>(a: T, b: U): T & U { return { ...a, ...b } }
```

```ds expected
function merge<T, U>(a: T, b: U): T & U {
    return { ...a, ...b };
}
```

### generic with default

Default type parameters use `= Type` syntax.

```ds
function create<T = any>(): T[] { return [] }
```

```ds expected
function create<T = any>(): T[] {
    return [];
}
```

### generic with comptime parameter

Comptime static parameters keep the keyword in the parameter list.

```ds
function repeat<comptime N: int>(value: string): string { return value }
```

```ds expected
function repeat<comptime N: int>(value: string): string {
    return value;
}
```

## Parameters

### optional parameter

Optional parameters use `?` after the parameter name.

```ds
function greet(name?: string) { return `Hello, ${name ?? "world"}` }
```

```ds expected
function greet(name?: string) {
    return `Hello, ${name ?? "world"}`;
}
```

### default parameter

Default values use `= value` after the type annotation.

```ds
function greet(name: string = "world") { return `Hello, ${name}` }
```

```ds expected
function greet(name: string = "world") {
    return `Hello, ${name}`;
}
```

### rest parameter

Rest parameters use `...` prefix and must be the last parameter.

```ds
function sum(...numbers: number[]): number { return numbers.reduce((a, b) => a + b, 0) }
```

```ds expected
function sum(...numbers: number[]): number {
    return numbers.reduce((a, b) => a + b, 0);
}
```

### function with this parameter

Explicit `this` parameters stay first in the list.

```ts:main.ts
function bind(this: Handler, event: Event) { this.handle(event) }
```

```ts expected
function bind(this: Handler, event: Event) {
    this.handle(event);
}
```

### destructured parameter

Object destructuring in parameters preserves the pattern structure.

```ds
function point({ x, y }: Point): string { return `(${x}, ${y})` }
```

```ds expected
function point({ x, y }: Point): string {
    return `(${x}, ${y})`;
}
```

### array destructured parameter

Array destructuring extracts elements by position.

```ds
function first([head]: number[]): number { return head }
```

```ds expected
function first([head]: number[]): number {
    return head;
}
```

### parameter annotations

Annotated parameters keep the `@` prefix before the name.

```ds
function process(@nonempty input: string) { return input }
```

```ds expected
function process(@nonempty input: string) {
    return input;
}
```

## Line Breaking

### function with many parameters breaks

When parameters exceed the line width, they break to multiple lines with trailing comma.

```ds line-width=40
function foo(veryLongParam: string, anotherLongParam: number, thirdParam: boolean) { }
```

```ds expected
function foo(
    veryLongParam: string,
    anotherLongParam: number,
    thirdParam: boolean,
) {}
```

### generic function with many type params breaks

Type parameters also break when they exceed the line width.

```ds line-width=40
function foo<VeryLongType, AnotherLongType, ThirdType>(x: VeryLongType): void { }
```

```ds expected
function foo<
    VeryLongType,
    AnotherLongType,
    ThirdType,
>(x: VeryLongType): void {}
```

### function with where clause

Where clauses specify additional type constraints.

```ds
function process<T>(x: T): T where T: Copy { return x }
```

```ds expected
function process<T>(x: T): T where T: Copy {
    return x;
}
```

### function with multiple where constraints

Multiple where constraints can be grouped in parentheses.

```ds line-width=50
function process<T, U>(a: T, b: U): void where (T: Copy, U: Clone) { }
```

When the signature is too long, the where clause breaks to its own line.

```ds expected
function process<T, U>(a: T, b: U): void
where (T: Copy, U: Clone) {}
```

## Export and Visibility

### exported function

The `export` keyword precedes the function declaration.

```ds
export function foo() { }
```

```ds expected
export function foo() {}
```

### export default function

Default exports use `export default` before the function.

```ds
export default function handler() { }
```

```ds expected
export default function handler() {}
```

## Decorators

### decorated function

Function decorators appear on their own line above the function.

```ds
@deprecated("use newFoo")
function oldFoo() { }
```

```ds expected
@deprecated("use newFoo")
function oldFoo() {}
```

### multiple decorators

Multiple decorators each get their own line, in order.

```ds
@log
@memoize
function compute(x: number): number { return x * 2 }
```

```ds expected
@log
@memoize
function compute(x: number): number {
    return x * 2;
}
```

### decorator with arguments

Decorator arguments follow function call formatting rules.

```ds
@route("/api/users", { method: "GET" })
async function getUsers() { }
```

```ds expected
@route("/api/users", { method: "GET" })
async function getUsers() {}
```

## Overloads

### function overloads

Overload signatures are listed before the implementation signature.

```ds
function parse(x: string): number
function parse(x: number): number
function parse(x: string | number): number { return typeof x === "string" ? parseInt(x) : x }
```

```ds expected
function parse(x: string): number;
function parse(x: number): number;
function parse(x: string | number): number {
    return typeof x === "string" ? parseInt(x) : x;
}
```

## Type Predicates (TypeScript)

### type predicate return type

Type predicate return types keep `is` spacing.

```ts:main.ts
function isFoo(value: unknown): value is Foo { return value instanceof Foo }
```

```ts expected
function isFoo(value: unknown): value is Foo {
    return value instanceof Foo;
}
```

### type predicate with contextual type subject

Type predicate subjects can use names that are contextual type literals elsewhere.

```ts:main.d.ts
declare function isAnyArrayBuffer(object: unknown): object is ArrayBufferLike
```

```ts expected
declare function isAnyArrayBuffer(object: unknown): object is ArrayBufferLike;
```

### asserts type predicate return type

Asserted type predicates keep `asserts` and `is` spacing.

```ts:main.ts
function assertFoo(value: Foo): asserts value is Foo { return value !== null }
```

```ts expected
function assertFoo(value: Foo): asserts value is Foo {
    return value !== null;
}
```

### asserts type predicate comments

Comments before and after `is` stay inside the asserted predicate.

```ds
function assertFoo(value: unknown): asserts value /* value */ is /* type */ Foo { return }
```

```ds expected
function assertFoo(value: unknown): asserts value /* value */ is /* type */ Foo {
    return;
}
```

### asserts subject without predicate

Asserted subjects without predicates keep the `asserts` keyword.

```ts:main.ts
function assertDefined(value: Foo | null): asserts value { if (value === null) throw new Error() }
```

```ts expected
function assertDefined(value: Foo | null): asserts value {
    if (value === null) throw new Error();
}
```

## Comments

### function with doc comment

Doc comments are preserved above the function declaration.

```ds
/// Adds two numbers together.
function add(a: number, b: number): number { return a + b }
```

```ds expected
/// Adds two numbers together.
function add(a: number, b: number): number {
    return a + b;
}
```

### function with inline comment

Inline comments at the start of a block move to their own line.

```ds
function foo() { // inline comment
    return 1
}
```

```ds expected
function foo() {
    // inline comment
    return 1;
}
```
