# Formatter Ignores

## Format Ignore Comments

### prettier ignore preserves statement formatting

Prettier ignore keeps the next statement verbatim.

```tspp:main.tspp
// prettier-ignore
Object . defineProperties    (    exports    , { } );
```

```tspp expected
// prettier-ignore
Object . defineProperties    (    exports    , { } );
```

### format ignore preserves statement formatting

Format ignore keeps the next statement verbatim.

```tspp:main.tspp
// format-ignore
Object . defineProperties    (    exports    , { } );
```

```tspp expected
// format-ignore
Object . defineProperties    (    exports    , { } );
```

### block ignore preserves expression formatting

Block ignore comments keep the next expression unchanged.

```tspp:main.tspp
/* prettier-ignore */
(() =>
  c +
    b +
  d
);
```

```tspp expected
/* prettier-ignore */
(() =>
  c +
    b +
  d
);
```

### biome ignore preserves statement formatting

Biome ignore comments keep the next statement verbatim.

```tspp:main.tspp
// biome-ignore format: keep spacing
foo ( 1 , 2 );
```

```tspp expected
// biome-ignore format: keep spacing
foo ( 1 , 2 );
```

### formatter-specific ignore preserves statement formatting

Formatter-specific ignore comments keep the next statement verbatim.

```tspp:main.tspp
// oxfmt-ignore
console . error( "hi" );
```

```tspp expected
// oxfmt-ignore
console . error( "hi" );
```

### deno fmt ignore preserves statement formatting

Deno ignore comments keep the next statement verbatim.

```tspp:main.tspp
// deno-fmt-ignore
console . error( "hi" );
```

```tspp expected
// deno-fmt-ignore
console . error( "hi" );
```

### prettier ignore preserves object property formatting

Prettier ignore keeps a single property verbatim inside objects.

```tspp:main.tspp
const obj = {
    // prettier-ignore
    foo   :    bar,
    baz: 1,
};
```

```tspp expected
const obj = {
    // prettier-ignore
    foo   :    bar,
    baz: 1,
};
```

### prettier ignore preserves class member formatting

Prettier ignore keeps a class member verbatim.

```tspp:main.tspp
class Foo {
    // prettier-ignore
    bar   :    number;
    baz: number;
}
```

```tspp expected
class Foo {
    // prettier-ignore
    bar   :    number;
    baz: number;
}
```

### fmt ignore range preserves statement formatting

Fmt ignore range keeps the statements between start and end verbatim.

```tspp:main.tspp
// fmt-ignore-start
const foo   = 1;
const bar=2;
// fmt-ignore-end
const baz = 3;
```

```tspp expected
// fmt-ignore-start
const foo   = 1;
const bar=2;
// fmt-ignore-end
const baz = 3;
```

### format ignore range preserves statement formatting

Format ignore range keeps the statements between start and end verbatim.

```tspp:main.tspp
// format-ignore-start
const left   = 1;
const right=2;
// format-ignore-end
const done = true;
```

```tspp expected
// format-ignore-start
const left   = 1;
const right=2;
// format-ignore-end
const done = true;
```

### format ignore range preserves object members

Format ignore range keeps object members verbatim.

```tspp:main.tspp
const obj = {
    foo: 1,
    // format-ignore-start
    bar   :    baz,
    qux:2,
    // format-ignore-end
    zap: 3,
};
```

```tspp expected
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

```tspp:main.tspp
class Foo {
    bar: number;
    // format-ignore-start
    baz   :    number;
    qux:number;
    // format-ignore-end
    zap: number;
}
```

```tspp expected
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

```tspp:main.tspp
const values = [
    1,
    // format-ignore-start
    foo ( 1 ,2 ),
    bar(3),
    // format-ignore-end
    baz(4),
]
```

```tspp expected
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

```tspp:main.tspp
doThing(
    1,
    // format-ignore-start
    foo ( 1 ,2 ),
    bar(3),
    // format-ignore-end
    4,
)
```

```tspp expected
doThing(
    1,
    // format-ignore-start
    foo ( 1 ,2 ),
    bar(3),
    // format-ignore-end
    4,
);
```
