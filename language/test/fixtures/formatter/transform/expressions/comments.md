# Comment Formatting

Expression comment fixtures cover inline comments, ignore directives, and ignored ranges.

## Inline Comments

### comment before argument

Comments before arguments stay inline when the call still fits.

```ds
foo(/* first */ a, /* second */ b)
```

```ds expected
foo(/* first */ a, /* second */ b);
```

### comment in array elements

Comments inside arrays cause expansion to multiline format.

```ds
[/* first */ 1, /* second */ 2, /* third */ 3]
```

```ds expected
[/* first */ 1, /* second */ 2, /* third */ 3];
```

### comment in binary expression

Comments in binary expressions keep normalized spacing.

```ds line-width=100
const x = /* pre-A */ A /* A comment */ && B /* B comment */
```

```ds expected
const x = /* pre-A */ A /* A comment */ && B; /* B comment */
```

## Comments Causing Expansion

### comment in object stays inline when short

Short objects with internal comments stay inline.

```ts:main.ts
({ /* key */ a: 1, /* another */ b: 2 })
```

The formatter keeps this object inline when it fits.

```ts expected
({ /* key */ a: 1, /* another */ b: 2 });
```

### comment in computed object key

Comments before computed keys also expand objects to multiple lines.

```ts:main.ts
({ /* key */ [k]: value })
```

```ts expected
({ /* key */ [k]: value });
```

### comment in function body

Comments in function bodies are preserved.

```ds
function foo() { /* empty */ }
```

```ds expected
function foo() {
    /* empty */
}
```

## Block Comments

### multiline block comment preservation

Multiline block comments are preserved with formatting.

```ds
{
    /*
     * Comment 1
     */
    const x = 1
}
```

```ds expected
{
    /*
     * Comment 1
     */
    const x = 1;
}
```

### doc comment on declaration

Doc comments precede declarations.

```ds
{
    /** some multiline
     * doc comment
     * over multiple lines */
    const X = 1
}
```

```ds expected
{
    /** some multiline
     * doc comment
     * over multiple lines */
    const X = 1;
}
```

## Line Comments

### line comment after statement

Line comments after statements are preserved.

```ds
{
    const x = 1; // important value
    const y = 2; // another value
}
```

```ds expected
{
    const x = 1; // important value
    const y = 2; // another value
}
```

### multiple line comments before declaration

Multiple consecutive line comments are preserved.

```ds
{
    // comment part 1
    // comment part 2
    const A = 1
}
```

```ds expected
{
    // comment part 1
    // comment part 2
    const A = 1;
}
```

## Comments in Logical Operators

### comment between logical and operands

Comments between logical operators can cause expansion when they add visual weight.

```ds line-width=60
const valid = isActive() && /* must have permission */ hasPermission()
```

```ds expected
const valid =
    isActive() &&
    /* must have permission */ hasPermission();
```

### comments in multiline logical chain

When logical chains break, comments stay with their operands.

```ds line-width=40
const valid = isActive() && /* perm */ hasPermission() && /* not blocked */ !isBlocked()
```

```ds expected
const valid =
    isActive() &&
    /* perm */ hasPermission() &&
    /* not blocked */ !isBlocked();
```

### comment in nullish coalescing

Comments in nullish coalescing expressions.

```ds
const value = input ?? /* fallback */ defaultValue
```

```ds expected
const value = input ?? /* fallback */ defaultValue;
```

## Comments in Ternary Expressions

### comment before ternary branches

Comments before ternary branches are preserved.

```ds line-width=60
const x = condition ? /* then */ valueA : /* else */ valueB
```

```ds expected
const x = condition ? /* then */ valueA : /* else */ valueB;
```

### comment in breaking ternary

Comments preserved when ternary breaks across lines.

```ds line-width=30
const x = condition ? /* yes */ valueA : /* no */ valueB
```

```ds expected
const x = condition
    ? /* yes */ valueA
    : /* no */ valueB;
```

## Comments in Assignments

### comment in chained assignment

Comments in chained assignments are preserved.

```ds
x = /* important */ y = /* also important */ z
```

```ds expected
x = /* important */ y = /* also important */ z;
```

### comment before assignment value

Comment between equals and value.

```ds
const result = /* computed */ calculate(a, b)
```

```ds expected
const result = /* computed */ calculate(a, b);
```

## Comments in Member Access

### comment in method chain

Comments between method calls in chains are preserved.

```ds
obj.method() /* step 1 */ .transform() /* step 2 */ .result()
```

```ds expected
obj.method() /* step 1 */
    .transform() /* step 2 */
    .result();
```

### comment before method call

When chains break, comments stay with their associated element.

```ds line-width=50
data.filter(x => x.valid) /* now map */ .map(x => x.value)
```

```ds expected
data.filter((x) => x.valid) /* now map */
    .map((x) => x.value);
```

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
