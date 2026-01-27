# Namespace Overloads

Namespace imports should preserve overload sets and overload order.
Reexports and export stars should not reorder overloads when accessed through a namespace.

## Functions

### namespace imports select the first overload

> Overload order should remain stable through `import * as ns`.

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
import * as ns from "./barrel";

const selected = ns.pick("x");
selected satisfies "general";
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "allowTs": true, "checkTs": true } }
```

### namespace imports do not select later overloads

> Later overload results should not be selected through `import * as ns`.

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
import * as ns from "./barrel";

const selected = ns.pick("x");
selected satisfies "specific";
```

- contains: not assignable

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "allowTs": true, "checkTs": true } }
```

## Methods

### namespace imported methods select the first overload

> Method overload order should remain stable through namespace imports.

```ts:api.ts
export class Parser {
    parse(value: string): "general";
    parse(value: "x"): "specific";
    parse(value: string): "general" | "specific" {
        return "general";
    }
}
```

```ts:main.ts
import * as ns from "./api";

const parser = new ns.Parser();
const selected = parser.parse("x");
selected satisfies "general";
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "allowTs": true, "checkTs": true } }
```

### namespace imported methods do not select later overloads

> Later method overload results should not be selected through namespace imports.

```ts:api.ts
export class Parser {
    parse(value: string): "general";
    parse(value: "x"): "specific";
    parse(value: string): "general" | "specific" {
        return "general";
    }
}
```

```ts:main.ts
import * as ns from "./api";

const parser = new ns.Parser();
const selected = parser.parse("x");
selected satisfies "specific";
```

- contains: not assignable

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "allowTs": true, "checkTs": true } }
```
