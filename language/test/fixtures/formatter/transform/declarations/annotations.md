# Declaration Annotations

Declaration annotation fixtures cover decorator placement, type annotations, and declaration syntax edges.

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

```ts:main.ts indent-width=2 line-width=80 quote-style=double
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

## Functions

### parameter annotation stays attached

Parameter annotations stay attached to the parameter.

```ds
function process(@nonempty input: string) { return input }
```

```ds expected
function process(@nonempty input: string) {
    return input;
}
```

### return type decorator annotation stays inline

Decorator prefixed return types stay attached after `:`.

```ds
function build(value: Buffer): @addrspace("shared") &Buffer { return value }
```

```ds expected
function build(value: Buffer): @addrspace("shared") &Buffer {
    return value;
}
```

### TypeScript return type decorator annotation stays inline

Decorator prefixed return types stay attached after `:` under non-default formatter options.

```ts:main.ts indent-width=2 line-width=80 quote-style=double
{
    function build(value: Buffer): @addrspace("shared") &Buffer { return value; }
}
```

```ts expected
{
  function build(value: Buffer): @addrspace("shared") &Buffer {
    return value;
  }
}
```

### method body boundary comment

Comments between method signatures and bodies stay at the boundary.

```ts:main.ts
class Box {
  run(): number // method-body
  {
    return 1
  }
}
```

```ts expected
class Box {
    run(): number {
        // method-body
        return 1;
    }
}
```

## Classes

### class field trailing block comment

Trailing block comments on class fields stay with the same field.

```ts:main.ts
class Box {
  first = 1 /* first-tail */
  second = 2
}
```

```ts expected
class Box {
    first = 1; /* first-tail */
    second = 2;
}
```

### decorated field stays on its own line

Body level field decorators stay on their own line above the field.

```ts:main.ts
class Box {
  @observable
  value: number
}
```

```ts expected
class Box {
    @observable
    value: number;
}
```

### decorated method stays on its own line

Body level method decorators stay on their own line above the method.

```ts:main.ts
class Box {
  @memoize
  compute(): number { return 1 }
}
```

```ts expected
class Box {
    @memoize
    compute(): number {
        return 1;
    }
}
```

### decorated accessor stays on its own line

Accessor decorators stay on their own line above the accessor.

```ts:main.ts
class Box {
  @observable accessor value: number
}
```

```ts expected
class Box {
    @observable
    accessor value: number;
}
```

### interleaved method decorator comments stay in order

Comments between stacked method decorators stay interleaved with the same decorator group.

```ts:main.ts
class Box {
  // comment before entity
  @entity
  // comment after entity
  // comment before foo
  @foo(1, 2, 3)
  // comment after foo
  method() {}
}
```

```ts expected
class Box {
    // comment before entity
    @entity
    // comment after entity
    // comment before foo
    @foo(1, 2, 3)
    // comment after foo
    method() {}
}
```

## Interfaces and Method Types

### interface method parameter trailing comment

Trailing comments inside method parameter lists stay attached to the same parameter.

```ts:main.ts
interface Worker {
  run(
    value: string, // value-tail
  ): number
}
```

```ts expected
interface Worker {
    run(
        value: string, // value-tail
    ): number;
}
```

### interface method return boundary comment

Boundary comments around method return types stay attached to the same method signature.

```ts:main.ts
interface Worker {
  run(): // return-tail
  Promise<void>
}
```

```ts expected
interface Worker {
    run(): // return-tail
    Promise<void>;
}
```

## Annotation Syntax Edges

### private accessor hash type annotation

Accessor hash members with type annotations are parsed and formatted.

```ts:main.ts
class Foo {
  accessor #p: any;
}
```

```ts expected
class Foo {
    accessor #p: any;
}
```

### abstract declare accessor type annotation

Abstract accessor fields with type annotations are parsed and formatted.

```ts:main.ts
abstract class Foo {
  abstract accessor prop7: number;
}
```

```ts expected
abstract class Foo {
    abstract accessor prop7: number;
}
```

### decorator class expression as superclass

Decorated class expressions are parsed in superclass positions.

```js:main.js
class Outer extends
  @deco
  class {} {}
```

```js expected
class Outer extends (
    @deco
    class {}
) {}
```

### TypeScript accessor modifiers with decorators

Accessor fields with mixed modifiers and hash members are parsed and formatted.

```ts:main.ts
abstract class Foo {
  abstract accessor prop7: number;
  accessor #p: any;
  accessor a!: any;
}
```

```ts expected
abstract class Foo {
    abstract accessor prop7: number;
    accessor #p: any;
    accessor a!: any;
}
```

## Assignment Boundaries

### assignment chain with ts-ignore marker

Assignment right side marker comments stay attached to the assigned expression.

```ts:main.ts
longVariableName1 = // @ts-ignore
(variable01 + veryLongVariableNameNumber2).method()
```

```ts expected
longVariableName1 = // @ts-ignore
    (variable01 + veryLongVariableNameNumber2).method();
```

### assignment to arrow with separator comments

Comments around assignment to arrow expressions stay attached to the assigned arrow.

```ts:main.ts
const handler = /* marker */

  // before-arrow

  () => {}
```

```ts expected
const handler =
    /* marker */

    // before-arrow

    () => {};
```

## Declaration Syntax Edges

### constructor parameter modifiers

Constructor parameter modifiers are parsed and formatted.

```ts:main.ts
class C {
  constructor(readonly x: number) {}
}

class D {
  constructor(public readonly x: number) {}
}

class E {
  constructor(private readonly x: number) {}
}
```

```ts expected
class C {
    constructor(readonly x: number) {}
}

class D {
    constructor(public readonly x: number) {}
}

class E {
    constructor(private readonly x: number) {}
}
```

### ambient declaration forms

Ambient declaration forms are parsed and formatted.

```ts:main.ts
declare type A = true;
declare function b(): "hello";
declare const foo: "bar";
declare var qux: boolean;
declare enum Kind {}
declare interface Shape {}
declare class Box {}
declare module Outer {}
declare module "named" {}
declare namespace Inner {}
```

```ts expected
declare type A = true;
declare function b(): "hello";
declare const foo: "bar";
declare var qux: boolean;
declare enum Kind {}
declare interface Shape {}
declare class Box {}
declare module Outer {}
declare module "named" {}
declare namespace Inner {}
```

### module declaration bodies

Module declaration bodies are parsed and formatted.

```ts:main.ts
module A {
  export class A {}
}

declare module "B" {
  export class B {}
}
```

```ts expected
module A {
    export class A {}
}

declare module "B" {
    export class B {}
}
```

### overload signatures with syntax edge parameters

Overload signatures with optional and rest parameters are parsed.

```ts:main.ts
function fn4a(x?: number, y: string)
function fn4a() {}

function fn5(x: string, y: string, ...rest: any[])
function fn5() {}
```

```ts expected
function fn4a(x?: number, y: string);
function fn4a() {}

function fn5(x: string, y: string, ...rest: any[]);
function fn5() {}
```

## Export and Interface Annotation Boundaries

### export declaration with boundary comments

Boundary comments around export declaration heads stay attached to the exported node.

```ts:main.ts
export // export-head
interface Shape {
  value: string // value-tail
}
```

```ts expected
export // export-head
interface Shape {
    value: string; // value-tail
}
```

### interface location comment boundaries

Comments around interface property and method type boundaries stay attached.

```ts:main.ts
interface Api {
  url: string // url-tail
  run(): // run-ret
  Promise<void>
}
```

```ts expected
interface Api {
    url: string; // url-tail
    run(): // run-ret
    Promise<void>;
}
```
