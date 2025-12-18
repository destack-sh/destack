# Comment Formatting

Tests for comments in various positions within expressions.

## Inline Comments

### comment before argument

Comments before arguments cause expansion to multiline format.

```ds
foo(/* first */ a, /* second */ b)
```

```ds expected
foo(
    /* first */ a,
    /* second */ b,
);
```

### comment in array elements

Comments inside arrays cause expansion to multiline format.

```ds
[/* first */ 1, /* second */ 2, /* third */ 3]
```

```ds expected
[
    /* first */ 1,
    /* second */ 2,
    /* third */ 3,
];
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

### _comment in object causes expansion

Objects with internal comments expand to multiple lines.

// #Broken: Parser doesn't handle comments before object properties yet.

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

## Comments in Logical Operators

### comment between logical and operands

Comments between logical operators can cause expansion when they add visual weight.

```ds line-width=60
const valid = isActive() && /* must have permission */ hasPermission()
```

```ds expected
const valid = isActive()
    && /* must have permission */ hasPermission();
```

### comments in multiline logical chain

When logical chains break, comments stay with their operands.

```ds line-width=40
const valid = isActive() && /* perm */ hasPermission() && /* not blocked */ !isBlocked()
```

```ds expected
const valid = isActive()
    && /* perm */ hasPermission()
    && /* not blocked */ !isBlocked();
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
obj.method() /* step 1 */ .transform() /* step 2 */ .result();
```

### comment before method call

When chains break, comments stay with their associated element.

```ds line-width=50
data.filter(x => x.valid) /* now map */ .map(x => x.value)
```

```ds expected
data
    .filter((x) => x.valid) /* now map */
    .map((x) => x.value);
```
