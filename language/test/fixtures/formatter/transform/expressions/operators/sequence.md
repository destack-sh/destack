# Sequence Expressions

Sequence fixtures cover comma expressions in expression and statement positions.

## Sequence Expressions

### unicode sequence in computed index

Sequence expressions inside computed indices are parenthesized.

```ts:main.ts
x = y['x・', 'x･']
```

```ts expected
x = y[("x・", "x･")];
```

### sequence expression statement

Sequence expressions format without extra parentheses in statements.

```ts:main.ts
a, b
```

```ts expected
(a, b);
```
