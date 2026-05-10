# Comment Primitives

Comment fixtures cover standalone comments, documentation comments, and formatter directives.

## Line Comments

### line comment on its own line

Comments on their own line are preserved in place.

```ds
// comment
const x = 1
```

The comment stays attached to the following statement.

```ds expected
// comment
const x = 1;
```

## Doc Comments

### doc comment before function

Doc comments using `///` are preserved before declarations.

```ds
/// This is a doc comment
function foo() { }
```

```ds expected
/// This is a doc comment
function foo() {}
```

## Comment Preservation

### preserves comment content exactly

Comment content is never modified by the formatter.

```ds
// XOXO: something something this later
const x = 1
```

```ds expected
// XOXO: something something this later
const x = 1;
```

### block doc comment with fenced code block indentation

Block doc comments format fenced code block indentation.

```ts:main.ts indent-style=tab
/**
 * Description text.
 *
 * ```ts
 * const store = toStore(
 * 	() => count,
 * 	(v) => (count = v),
 * );
 * ```
 */
function withFencedCodeBlock() {}
```

```ts expected
/**
 * Description text.
 *
 * ```ts
 * const store = toStore(
 * 	() => count,
 * 	(v) => (count = v),
 * );
 * ```
 */
function withFencedCodeBlock() {}
```

## Formatter Directives

### format-ignore keeps the next statement

Formatter ignore directives preserve the original statement formatting.

```ts:main.ts
// format-ignore
call(   a, b)
```

```ts expected
// format-ignore
call(   a, b)
```

### prettier-ignore keeps the next statement

Prettier ignore directives preserve the original statement formatting.

```ts:main.ts
// prettier-ignore
call(   a, b)
```

```ts expected
// prettier-ignore
call(   a, b)
```

### format-ignore range keeps multiple statements

Ignore ranges preserve the original formatting between the start and end markers.

```ts:main.ts
// format-ignore-start
const value  =   call(  1,2)
const other =    value +  1
// format-ignore-end
const ok = 1
```

```ts expected
// format-ignore-start
const value  =   call(  1,2)
const other =    value +  1
// format-ignore-end
const ok = 1;
```

### prettier-ignore range keeps multiple statements

Prettier ignore ranges preserve the original formatting between the start and end markers.

```ts:main.ts
// prettier-ignore-start
const value  =   call(  1,2)
const other =    value +  1
// prettier-ignore-end
const ok = 1
```

```ts expected
// prettier-ignore-start
const value  =   call(  1,2)
const other =    value +  1
// prettier-ignore-end
const ok = 1;
```

### biome-ignore format keeps the next statement

Biome format ignore directives preserve the original statement formatting.

```ts:main.ts
// biome-ignore format
call(   a, b)
```

```ts expected
// biome-ignore format
call(   a, b)
```

### TypeScript directive comments do not disable formatting

TypeScript diagnostic directives preserve the comment but still format code.

```ts:main.ts
// @ts-expect-error keep spacing
call(   a, b)

// @ts-ignore
value   =   compute(  1,  2)
```

```ts expected
// @ts-expect-error keep spacing
call(a, b);

// @ts-ignore
value = compute(1, 2);
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
    /** Some multiline doc comment over multiple lines */
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
