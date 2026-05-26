# Import Meta

Import meta fixtures cover `import.meta` member chains.

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
