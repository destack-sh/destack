# Comment Formatting

Tests for comments in various positions within expressions.

## Inline Comments

### comment before argument

Comments before arguments are preserved with spacing.

```ds
foo(/* first */ a, /* second */ b)
```

```ds expected
foo(/* first */ a, /* second */ b);
```

### comment in array elements

Comments inside arrays are preserved.

```ds
[/* first */ 1, /* second */ 2, /* third */ 3]
```

```ds expected
[/* first */ 1, /* second */ 2, /* third */ 3];
```

### comment in binary expression

Comments in binary expressions are preserved with proper spacing.

```ds line-width=100
const x = /* pre-A */ A /* A comment */ && B /* B comment */
```

```ds expected
const x = /* pre-A */ A /* A comment */ && B /* B comment */;
```

## Comments Causing Expansion

### comment in object causes expansion

Objects with internal comments expand to multiple lines.

```ds
{ /* key */ a: 1, /* another */ b: 2 }
```

The formatter expands the object when it contains comments.

```ds expected
{
    /* key */ a: 1,
    /* another */ b: 2,
};
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
    const x = 1
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
    const X = 1
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
    const A = 1
}
```
