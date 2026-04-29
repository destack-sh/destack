# Import Expressions

Import expression fixtures cover dynamic import calls and source comments.

## Import Calls

### import call expression

Import calls keep tight parentheses.

```ts:main.ts
const mod = import("module")
```

```ts expected
const mod = import("module");
```

### await import call

Awaited imports format like normal await expressions.

```ts:main.ts
const mod = await import("module")
```

```ts expected
const mod = await import("module");
```
