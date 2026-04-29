# Type Comments

Type comment fixtures cover comment ownership in unions, intersections, mapped types, conditionals, and signatures.

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

### union boundary comments stay attached

Union boundary comments stay attached to the same arms under non-default formatter options.

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

## Union and Intersection Layout

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

## Mapped Types and Ignore Boundaries

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

## Conditional and Intersection Type Comments

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
