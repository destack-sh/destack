# Cross Module Overload Order

Overload order should remain stable across exports, reexports, and export stars.
The first matching overload should still win after crossing module boundaries.

## Direct exports

### direct imports select the first overload

> Importing a function should not reorder its overload set.

```ts:api.ts
export function pick(value: string): "general";
export function pick(value: "x"): "specific";
export function pick(value: string): "general" | "specific" {
    return "general";
}
```

```ts:main.ts
import { pick } from "./api";

const selected = pick("x");
selected satisfies "general";
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true } }
```

### direct imports do not select later overloads

> Later overloads should not win after direct imports.

```ts:api.ts
export function pick(value: string): "general";
export function pick(value: "x"): "specific";
export function pick(value: string): "general" | "specific" {
    return "general";
}
```

```ts:main.ts
import { pick } from "./api";

const selected = pick("x");
selected satisfies "specific";
```

- not assignable

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true } }
```

## Reexports

### named reexports select the first overload

> Named reexports should forward overload order without modification.

```ts:api.ts
export function pick(value: string): "general";
export function pick(value: "x"): "specific";
export function pick(value: string): "general" | "specific" {
    return "general";
}
```

```ts:barrel.ts
export { pick } from "./api";
```

```ts:main.ts
import { pick } from "./barrel";

const selected = pick("x");
selected satisfies "general";
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true } }
```

### named reexports do not select later overloads

> Later overloads should not win after named reexports.

```ts:api.ts
export function pick(value: string): "general";
export function pick(value: "x"): "specific";
export function pick(value: string): "general" | "specific" {
    return "general";
}
```

```ts:barrel.ts
export { pick } from "./api";
```

```ts:main.ts
import { pick } from "./barrel";

const selected = pick("x");
selected satisfies "specific";
```

- not assignable

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true } }
```

### export stars select the first overload

> Export stars should not reorder overload sets.

```ts:api.ts
export function pick(value: string): "general";
export function pick(value: "x"): "specific";
export function pick(value: string): "general" | "specific" {
    return "general";
}
```

```ts:barrel.ts
export * from "./api";
```

```ts:main.ts
import { pick } from "./barrel";

const selected = pick("x");
selected satisfies "general";
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true } }
```

### export stars do not select later overloads

> Later overloads should not win after export stars.

```ts:api.ts
export function pick(value: string): "general";
export function pick(value: "x"): "specific";
export function pick(value: string): "general" | "specific" {
    return "general";
}
```

```ts:barrel.ts
export * from "./api";
```

```ts:main.ts
import { pick } from "./barrel";

const selected = pick("x");
selected satisfies "specific";
```

- not assignable

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true } }
```
