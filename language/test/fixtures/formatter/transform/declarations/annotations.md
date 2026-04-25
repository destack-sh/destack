# Declaration Annotations

Tests for annotation and comment boundaries in declarations.

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

### typescript decorator prefixed variable type stays inline

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

### typescript return type decorator annotation stays inline

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

## Interfaces And Method Types

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

## Type Declarations

### union doc block comment arm

Doc block comments in union expressions stay attached to the same type side.

```ts:main.ts line-width=80
export type Value = /** union-doc
 */
| { ok: true }
| { ok: false; value: bigint | null };
```

```ts expected
export type Value = /** union-doc
 */
{ ok: true } | { ok: false; value: bigint | null };
```

### union last arm trailing line comment

Trailing line comments on union last arms stay attached to that arm.

```ts:main.ts line-width=30
type Value =
  | First
  | Second // second-tail
```

```ts expected
type Value = First | Second; // second-tail
```

### union comment seams stay attached

Union seam comments stay attached to the same arms under non-default formatter options.

```ts:main.ts indent-width=2 line-width=80 quote-style=double
interface _KeywordDef {
  type?: JSONType | JSONType[] // data types that keyword applies to
}

type C1 = | (
  /* 1 */ /*1*/ | (
    | (
          | A
          // A comment to force break
          | B
        )
  )
  );

type C2 = | (
  /* 1 */ /*1*/
  /* 1 */ | (
    | (
          | A
          // A comment to force break
          | B
        )
  )
  );
```

```ts expected
interface _KeywordDef {
  type?: JSONType | JSONType[]; // data types that keyword applies to
}

type C1 = /* 1 */ /*1*/
  | A
  // A comment to force break
  | B;

type C2 =
  /* 1 */ /*1*/
  /* 1 */ | A
  // A comment to force break
  | B;
```

### union leading doc comment stays on the first arm

Leading doc comments on the first union arm stay attached to that arm under non-default formatter options.

```ts:main.ts indent-width=2 line-width=80 quote-style=double
export type AddressAllocator =
(/** Reserve a specific IP address. The pool is inferred from the address since IP pools cannot have overlapping ranges. */
| {
y: boolean
,}
| {
x: boolean }
);
```

```ts expected
export type AddressAllocator =
  /** Reserve a specific IP address. The pool is inferred from the address since IP pools cannot have overlapping ranges. */
  | {
      y: boolean;
    }
  | {
      x: boolean;
    };
```

### parenthesized union comment attachment

Comments inside parenthesized unions stay inside the same parentheses.

```ts:main.ts line-width=36
type Value = (First | // paren-union
Second) & Third
```

```ts expected
type Value = (
    | First // paren-union
    | Second
) &
    Third;
```

### mapped type property comments

Comments in mapped type bodies stay attached to the same property.

```ts:main.ts
type Flags<T> = {
  [K in keyof T]: // mapped-line
  boolean
}
```

```ts expected
type Flags<T> = {
    [K in keyof T]: boolean; // mapped-line
};
```

## Parser Coverage For Annotation Syntax

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

### typescript accessor modifiers with decorators

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

## Class Heritage And Generics

### class extends and implements with boundary comments

Comments around `extends` and `implements` boundaries stay attached to the same clause.

```ts:main.ts
class Derived extends Base // base-tail
implements
// impl-head
A,
B // impl-tail
{}
```

```ts expected
class Derived
    extends Base // base-tail
    // impl-head
    implements A, B {
    // impl-tail
}
```

### declare class with implements and generic comment

Comments on declare class generic and implements boundaries stay attached to declarations.

```ts:main.ts
declare class Box // box-head
<T> implements Item<T>, Other {
  value: T
}
```

```ts expected
declare class Box<T> // box-head
    implements Item<T>, Other
{
    value: T;
}
```

## Method Type Signatures

### interface optional method comments

Comments around optional method signatures stay attached to the same signature boundary.

```ts:main.ts
interface Methods {
  run/* name */ ? /* q */ (value: /* arg */ string): /* ret */ string
}
```

```ts expected
interface Methods {
    run /* name */? /* q */(value: /* arg */ string): /* ret */ string;
}
```

### callable and constructor type comments

Comments around callable and constructor type signatures stay attached to the signature node.

```ts:main.ts
type Fn = /* fn-head */ (value: /* arg */ string) /* fn-tail */ => void
let Factory: new /* ctor-head */ (value: /* arg */ string) /* ctor-tail */ => Widget;
```

```ts expected
type Fn = /* fn-head */ (value: /* arg */ string) /* fn-tail */ => void;
let Factory: new /* ctor-head */(value: /* arg */ string) /* ctor-tail */ => Widget;
```

## Union And Intersection Layout

### union with leading separators and last comments

Leading separator unions keep arm and final comments attached to the same arms.

```ts:main.ts
type Result = (
  | "a" // arm-a
  | "b" // arm-b
)[]; // final-tail
```

```ts expected
type Result = (
    | "a" // arm-a
    | "b" // arm-b
)[]; // final-tail
```

### parenthesized union in indexed access

Parenthesized unions in indexed access types keep parentheses and comments stable.

```ts:main.ts
type Key = (number | // key-note
string)["toString"]
```

```ts expected
type Key = (
    | number // key-note
    | string
)["toString"];
```

### union with inlined object arm comments

Object arm comments in inlined unions stay attached to the same arm.

```ts:main.ts
type Mixed = null // null-arm
| {
  y: number;
  z: string;
} // object-arm
| void // void-arm
;
```

```ts expected
type Mixed =
    | null // null-arm
    | {
          y: number;
          z: string;
      } // object-arm
    | void; // void-arm
```

## Parser Coverage For TypeScript Declarations

### constructor parameter modifiers

Constructor parameter modifier combinations are parsed and formatted.

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


## Mapped Types And Ignore Boundaries

### mapped type with ignore boundary comments

Ignore comments around mapped type boundaries stay attached to mapped type clauses.

```ts:main.ts
// prettier-ignore
type Value<T> = {
  [K in keyof T as // mapped-key
    `${K & string}`]: T[K]
}
```

```ts expected
// prettier-ignore
type Value<T> = {
  [K in keyof T as // mapped-key
    `${K & string}`]: T[K]
}
```

### mapped type with nested union comments

Nested union comments inside mapped types stay attached to the same union arm.

```ts:main.ts line-width=40
type Value<T> = {
  [K in keyof T]:
    | T[K] // arm-a
    | undefined // arm-b
}
```

```ts expected
type Value<T> = {
    [K in keyof T]:
        | T[K] // arm-a
        | undefined; // arm-b
};
```

## Export And Interface Annotation Boundaries

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

## Union Canonicalization Boundaries

### single type union with trailing comments

Single type unions keep stable comment ownership on the remaining type.

```ts:main.ts
type Value =
  | string // single-tail
;
```

```ts expected
type Value = string; // single-tail
```

### private in operator with typed member comment

Private `in` operator checks keep comments attached across declaration and expression boundaries.

```ts:main.ts
class C {
  #field = 1

  has(value: object) {
    return #field in value // private-in
  }
}
```

```ts expected
class C {
    #field = 1;

    has(value: object) {
        return #field in value; // private-in
    }
}
```

## Conditional And Intersection Type Comments

### conditional type boundary comments

Conditional type comments stay attached to extends and branch boundaries.

```ts:main.ts line-width=48
type Value<T> = T extends /* extends-note */ string
  ? /* true-note */ number
  : /* false-note */ boolean
```

```ts expected
type Value<T> =
    T extends /* extends-note */ string
        ? /* true-note */ number
        : /* false-note */ boolean;
```

### conditional type line comment stays with the consequent

Line comments after `?` stay attached to the consequent under non-default formatter options.

```ts:main.ts indent-width=2 line-width=80 quote-style=double
type A = B extends T
  ? // comment
    foo
  : bar;
```

```ts expected
type A = B extends T
  ? // comment
    foo
  : bar;
```

### nested conditional type comments stay attached

Nested multiline comments stay attached to the same conditional branches under non-default formatter options.

```ts:main.ts indent-width=2 line-width=80 quote-style=double
type T = test extends B
  ? /* comment
       comment
       comment
       comment
    */
    foo
  : test extends B
  ? /* comment
  comment
    comment */
    foo
  : bar;
```

```ts expected
type T = test extends B
  ? /* comment
       comment
       comment
       comment
    */
    foo
  : test extends B
    ? /* comment
  comment
    comment */
      foo
    : bar;
```

### intersection comment in flow-consistent style

Intersection comments in flow-consistent layouts stay attached to the same member boundary.

```ts:main.ts line-width=36
type Value = Left & // inter-note
Right & Tail
```

```ts expected
type Value = Left & // inter-note
    Right &
    Tail;
```

### tuple dangling type comments

Dangling comments in tuple type members stay attached to the same tuple position.

```ts:main.ts
type Pair = [
  string, // first-tail
  number // second-tail
]
```

```ts expected
type Pair = [
    string, // first-tail
    number, // second-tail
];
```

## Class Heritage Comments

### class superclass boundary comment

Comments around superclass boundaries stay attached to class heritage heads.

```ts:main.ts
class Child extends Base // extends-tail
{
  value = 1
}
```

```ts expected
class Child extends Base {
    // extends-tail
    value = 1;
}
```

### class implement list comment placement

Comments in implement lists stay attached to the same implement element.

```ts:main.ts
class Child implements First, // impl-first
Second // impl-second
{
  value = 1
}
```

```ts expected
class Child
    implements
        First, // impl-first
        Second
{
    // impl-second
    value = 1;
}
```

## Union Layout Comments

### union inlining with arm comments

Inlined unions keep arm comments attached after multiline expansion.

```ts:main.ts line-width=38
type Value = Alpha | // alpha-note
Beta | Gamma
```

```ts expected
type Value =
    | Alpha // alpha-note
    | Beta
    | Gamma;
```

### union with prettier ignore boundary

Prettier-ignore boundaries around unions keep the ignored union content stable.

```ts:main.ts
// prettier-ignore
type Value =
  | A // a-tail
  | B // b-tail
;
```

```ts expected
// prettier-ignore
type Value =
  | A // a-tail
  | B // b-tail
;
```

### union last comment boundary

Last union arm comments stay attached to the last arm.

```ts:main.ts
type Value =
  | A
  | B // last-union
```

```ts expected
type Value = A | B; // last-union
```

## Mapped Type Comments

### mapped type break mode comments

Mapped type break-mode comments stay attached to key and value boundaries.

```ts:main.ts line-width=40
type Flags<T> = {
  readonly [K in keyof T]?: // map-value
  boolean
}
```

```ts expected
type Flags<T> = {
    readonly [K in keyof T]?: boolean; // map-value
};
```

### mapped type remap with comment boundaries

Mapped type key remap comments stay attached to remap boundaries.

```ts:main.ts
type Paths<T> = {
  [K in keyof T as // remap-note
    `get${Capitalize<K & string>}`]: () => T[K]
}
```

```ts expected
type Paths<T> = {
    [K in keyof T as `get${Capitalize<K & string> // remap-note
    }`]: () => T[K];
};
```
