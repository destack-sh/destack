# Type Operator Comments

## Union Canonicalization Boundaries

### single type union with trailing comments

Single type unions keep stable comment ownership on the remaining type.

```ds:main.ds
type Value =
  | string // single-tail
;
```

```ds expected
type Value = string; // single-tail
```

## Type Declarations

### union doc block comment arm

Doc block comments in union expressions stay attached to the same type side.

```ds:main.ds line-width=80
export type Value = /** union-doc
 */
| { ok: true }
| { ok: false; value: bigint | null };
```

```ds expected
export type Value =
    /** union-doc
     */
    { ok: true } | { ok: false; value: bigint | null };
```

### union last arm trailing line comment

Trailing line comments on union last arms stay attached to that arm.

```ds:main.ds line-width=30
type Value =
  | First
  | Second // second-tail
```

```ds expected
type Value = First | Second; // second-tail
```

### union boundary comments stay attached

Union boundary comments stay attached to the same arms under non-default formatter options.

```ds:main.ds indent-width=2 line-width=80
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

```ds expected
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

```ds:main.ds indent-width=2 line-width=80
export type AddressAllocator =
(/** Reserve a specific IP address. The pool is inferred from the address since IP pools cannot have overlapping ranges. */
| {
y: boolean
,}
| {
x: boolean }
);
```

```ds expected
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

```ds:main.ds line-width=36
type Value = (First | // paren-union
Second) & Third
```

```ds expected
type Value = (
    | First // paren-union
    | Second
) &
    Third;
```

### mapped type property comments

Comments in mapped type bodies stay attached to the same property.

```ds:main.ds
type Flags<T> = {
  [K in keyof T]: // mapped-line
  boolean
}
```

```ds expected
type Flags<T> = {
    [K in keyof T]: boolean; // mapped-line
};
```

## Mapped Types and Ignore Boundaries

### mapped type with ignore boundary comments

Ignore comments around mapped type boundaries stay attached to mapped type clauses.

```ds:main.ds
// prettier-ignore
type Value<T> = {
  [K in keyof T as // mapped-key
    `${K & string}`]: T[K]
}
```

```ds expected
// prettier-ignore
type Value<T> = {
  [K in keyof T as // mapped-key
    `${K & string}`]: T[K]
}
```

### mapped type with nested union comments

Nested union comments inside mapped types stay attached to the same union arm.

```ds:main.ds line-width=40
type Value<T> = {
  [K in keyof T]:
    | T[K] // arm-a
    | undefined // arm-b
}
```

```ds expected
type Value<T> = {
    [K in keyof T]:
        | T[K] // arm-a
        | undefined; // arm-b
};
```


## Mapped Type Comments

### mapped type break mode comments

Mapped type break-mode comments stay attached to key and value boundaries.

```ds:main.ds line-width=40
type Flags<T> = {
  readonly [K in keyof T]?: // map-value
  boolean
}
```

```ds expected
type Flags<T> = {
    readonly [K in keyof T]?: boolean; // map-value
};
```

### mapped type remap with comment boundaries

Mapped type key remap comments stay attached to remap boundaries.

```ds:main.ds
type Paths<T> = {
  [K in keyof T as // remap-note
    `get${Capitalize<K & string>}`]: () => T[K]
}
```

```ds expected
type Paths<T> = {
    [K in keyof T as // remap-note
        `get${Capitalize<K & string>}`]: () => T[K];
};
```

## Method Type Signatures

### interface optional method comments

Comments around optional method signatures stay attached to the same signature boundary.

```ds:main.ds
interface Methods {
  run/* name */ ? /* q */ (value: /* arg */ string): /* ret */ string
}
```

```ds expected
interface Methods {
    run /* name */? /* q */(value: /* arg */ string): /* ret */ string;
}
```

### callable and constructor type comments

Comments around callable and constructor type signatures stay attached to the signature node.

```ds:main.ds
type Fn = /* fn-head */ (value: /* arg */ string) /* fn-tail */ => void
let Factory: new /* ctor-head */ (value: /* arg */ string) /* ctor-tail */ => Widget;
```

```ds expected
type Fn = /* fn-head */ (value: /* arg */ string) /* fn-tail */ => void;
let Factory: new /* ctor-head */(value: /* arg */ string) /* ctor-tail */ => Widget;
```

## Union and Intersection Comments

### union with leading separators and last comments

Leading separator unions keep arm and final comments attached to the same arms.

```ds:main.ds
type Result = (
  | "a" // arm-a
  | "b" // arm-b
)[]; // final-tail
```

```ds expected
type Result = (
    | "a" // arm-a
    | "b" // arm-b
)[]; // final-tail
```

### parenthesized union in indexed access

Parenthesized unions in indexed access types keep parentheses and comments stable.

```ds:main.ds
type Key = (number | // key-note
string)["toString"]
```

```ds expected
type Key = (
    | number // key-note
    | string
)["toString"];
```

### union with inlined object arm comments

Object arm comments in inlined unions stay attached to the same arm.

```ds:main.ds
type Mixed = null // null-arm
| {
  y: number;
  z: string;
} // object-arm
| void // void-arm
;
```

```ds expected
type Mixed =
    | null // null-arm
    | {
          y: number;
          z: string;
      } // object-arm
    | void; // void-arm
```


## Conditional and Intersection Type Comments

### conditional type boundary comments

Conditional type comments stay attached to extends and branch boundaries.

```ds:main.ds line-width=48
type Value<T> = T extends /* extends-note */ string
  ? /* true-note */ number
  : /* false-note */ boolean
```

```ds expected
type Value<T> =
    T extends /* extends-note */ string
        ? /* true-note */ number
        : /* false-note */ boolean;
```

### conditional type line comment stays with the consequent

Line comments after `?` stay attached to the consequent under non-default formatter options.

```ds:main.ds indent-width=2 line-width=80
type A = B extends T
  ? // comment
    foo
  : bar;
```

```ds expected
type A = B extends T
  ? // comment
    foo
  : bar;
```

### nested conditional type comments stay attached

Nested multiline comments stay attached to the same conditional branches under non-default formatter options.

```ds:main.ds indent-width=2 line-width=80
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

```ds expected
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

```ds:main.ds line-width=36
type Value = Left & // inter-note
Right & Tail
```

```ds expected
type Value = Left & // inter-note
    Right &
    Tail;
```

### tuple dangling type comments

Dangling comments in tuple type members stay attached to the same tuple position.

```ds:main.ds
type Pair = (
  string, // first-tail
  number // second-tail
)
```

```ds expected
type Pair = (
    string, // first-tail
    number, // second-tail
);
```

## Union Arm Comments

### union inlining with arm comments

Inlined unions keep arm comments attached after multiline expansion.

```ds:main.ds line-width=38
type Value = Alpha | // alpha-note
Beta | Gamma
```

```ds expected
type Value =
    | Alpha // alpha-note
    | Beta
    | Gamma;
```

### union with prettier ignore boundary

Prettier-ignore boundaries around unions keep the ignored union content stable.

```ds:main.ds
// prettier-ignore
type Value =
  | A // a-tail
  | B // b-tail
;
```

```ds expected
// prettier-ignore
type Value =
  | A // a-tail
  | B // b-tail
;
```

### union last comment boundary

Last union arm comments stay attached to the last arm.

```ds:main.ds
type Value =
  | A
  | B // last-union
```

```ds expected
type Value = A | B; // last-union
```
