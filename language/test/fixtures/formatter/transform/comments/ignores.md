# Formatter Ignores

## Format Ignore Comments

### prettier ignore preserves statement formatting

Prettier ignore keeps the next statement verbatim.

```ts:main.ts
// prettier-ignore
Object . defineProperties    (    exports    , { } );
```

```ts expected
// prettier-ignore
Object . defineProperties    (    exports    , { } );
```

### format ignore preserves statement formatting

Format ignore keeps the next statement verbatim.

```ts:main.ts
// format-ignore
Object . defineProperties    (    exports    , { } );
```

```ts expected
// format-ignore
Object . defineProperties    (    exports    , { } );
```

### block ignore preserves expression formatting

Block ignore comments keep the next expression unchanged.

```ts:main.ts
/* prettier-ignore */
(() =>
  c +
    b +
  d
);
```

```ts expected
/* prettier-ignore */
(() =>
  c +
    b +
  d
);
```

### biome ignore preserves statement formatting

Biome ignore comments keep the next statement verbatim.

```ts:main.ts
// biome-ignore format: keep spacing
foo ( 1 , 2 );
```

```ts expected
// biome-ignore format: keep spacing
foo ( 1 , 2 );
```

### formatter-specific ignore preserves statement formatting

Formatter-specific ignore comments keep the next statement verbatim.

```ts:main.ts
// oxfmt-ignore
console . error( "hi" );
```

```ts expected
// oxfmt-ignore
console . error( "hi" );
```

### deno fmt ignore preserves statement formatting

Deno ignore comments keep the next statement verbatim.

```ts:main.ts
// deno-fmt-ignore
console . error( "hi" );
```

```ts expected
// deno-fmt-ignore
console . error( "hi" );
```

### prettier ignore preserves object property formatting

Prettier ignore keeps a single property verbatim inside objects.

```ts:main.ts
const obj = {
    // prettier-ignore
    foo   :    bar,
    baz: 1,
};
```

```ts expected
const obj = {
    // prettier-ignore
    foo   :    bar,
    baz: 1,
};
```

### prettier ignore preserves class member formatting

Prettier ignore keeps a class member verbatim.

```ts:main.ts
class Foo {
    // prettier-ignore
    bar   :    number;
    baz: number;
}
```

```ts expected
class Foo {
    // prettier-ignore
    bar   :    number;
    baz: number;
}
```

### fmt ignore range preserves statement formatting

Fmt ignore range keeps the statements between start and end verbatim.

```ts:main.ts
// fmt-ignore-start
const foo   = 1;
const bar=2;
// fmt-ignore-end
const baz = 3;
```

```ts expected
// fmt-ignore-start
const foo   = 1;
const bar=2;
// fmt-ignore-end
const baz = 3;
```

### format ignore range preserves statement formatting

Format ignore range keeps the statements between start and end verbatim.

```ts:main.ts
// format-ignore-start
const left   = 1;
const right=2;
// format-ignore-end
const done = true;
```

```ts expected
// format-ignore-start
const left   = 1;
const right=2;
// format-ignore-end
const done = true;
```

### format ignore range preserves object members

Format ignore range keeps object members verbatim.

```ts:main.ts
const obj = {
    foo: 1,
    // format-ignore-start
    bar   :    baz,
    qux:2,
    // format-ignore-end
    zap: 3,
};
```

```ts expected
const obj = {
    foo: 1,
    // format-ignore-start
    bar   :    baz,
    qux:2,
    // format-ignore-end
    zap: 3,
};
```

### format ignore range preserves class members

Format ignore range keeps class members verbatim.

```ts:main.ts
class Foo {
    bar: number;
    // format-ignore-start
    baz   :    number;
    qux:number;
    // format-ignore-end
    zap: number;
}
```

```ts expected
class Foo {
    bar: number;
    // format-ignore-start
    baz   :    number;
    qux:number;
    // format-ignore-end
    zap: number;
}
```

### format ignore range preserves array elements

Format ignore range keeps array elements verbatim.

```ts:main.ts
const values = [
    1,
    // format-ignore-start
    foo ( 1 ,2 ),
    bar(3),
    // format-ignore-end
    baz(4),
]
```

```ts expected
const values = [
    1,
    // format-ignore-start
    foo ( 1 ,2 ),
    bar(3),
    // format-ignore-end
    baz(4),
];
```

### format ignore range preserves call arguments

Format ignore range keeps call arguments verbatim.

```ts:main.ts
doThing(
    1,
    // format-ignore-start
    foo ( 1 ,2 ),
    bar(3),
    // format-ignore-end
    4,
)
```

```ts expected
doThing(
    1,
    // format-ignore-start
    foo ( 1 ,2 ),
    bar(3),
    // format-ignore-end
    4,
);
```
