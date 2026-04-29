# Meta Expressions

Meta expression fixtures cover `import.meta` and `new.target` member chains.

## Import Meta

### import.meta property access

Import meta property access keeps dots tight.

```ts:main.ts
const url = import.meta.url
```

```ts expected
const url = import.meta.url;
```

### import.meta nested property access

Nested import meta property access stays as a normal chain.

```ts:main.ts
const mode = import.meta.env.MODE
```

```ts expected
const mode = import.meta.env.MODE;
```

## New Target

### new.target property access

New target property access keeps dots tight.

```ts:main.ts
const target = new.target
```

```ts expected
const target = new.target;
```
