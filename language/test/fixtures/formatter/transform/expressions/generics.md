# Generics

Tests for generic type argument formatting.

## Type Arguments

### generic brackets have no internal spacing

Spaces inside angle brackets should be removed.

```ds
const x: Array< number > = []
```

```ds expected
const x: Array<number> = [];
```

### generic with multiple parameters

Multiple type parameters are separated by comma and space.

```ds
const x: Map< string , number > = new Map()
```

```ds expected
const x: Map<string, number> = new Map();
```

### generic with comments

Comments inside type arguments are preserved.

```ds
const x: Map</* key */ string, /* value */ number> = new Map()
```

```ds expected
const x: Map</* key */ string, /* value */ number> = new Map();
```

## Instantiation Expressions

### instantiation keeps type arguments inline

Instantiation expressions should retain their type arguments without extra spacing.

```ts:main.ts
const factory = getFactory<number>
```

```ts expected
const factory = getFactory<number>;
```

### instantiation with multiple parameters

Multiple type arguments are separated by comma and space.

```ts:main.ts
const pair = makePair<string, number>
```

```ts expected
const pair = makePair<string, number>;
```

### instantiation with comments

Comments inside instantiation type arguments are preserved.

```ts:main.ts
const pair = makePair</* key */ string, /* value */ number>
```

```ts expected
const pair = makePair</* key */ string, /* value */ number>;
```

### instantiation with multiline comments

Comments inside multiline instantiation type arguments keep the type arguments multiline.

```ts:main.ts
Math.random<
  // comment
  string | number | undefined
>
```

```ts expected
Math.random<
    // comment
    string | number | undefined
>;
```
