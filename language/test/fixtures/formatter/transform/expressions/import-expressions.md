# Import Expressions

Tests for dynamic import expression formatting.

## Basic Import Call

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
