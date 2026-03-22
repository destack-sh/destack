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

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true } }
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

- not assignable

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true } }
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

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true } }
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

- not assignable

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true } }
```

### namespace imports preserve overload order through renamed re-exports

> Namespace access through renamed re-export chains should preserve overload order.

```ts:api.ts
export function pick(value: string): "general";
export function pick(value: "x"): "specific";
export function pick(value: string): "general" | "specific" {
    return "general";
}
```

```ts:index.ts
export { pick as choose } from "./api";
```

```ts:main.ts
import * as ns from "./index";

const selected = ns.choose("x");
selected satisfies "general";
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true } }
```

### namespace imports through renamed re-exports do not select later overloads

> Renamed namespace access should not promote later overload results.

```ts:api.ts
export function pick(value: string): "general";
export function pick(value: "x"): "specific";
export function pick(value: string): "general" | "specific" {
    return "general";
}
```

```ts:index.ts
export { pick as choose } from "./api";
```

```ts:main.ts
import * as ns from "./index";

const selected = ns.choose("x");
selected satisfies "specific";
```

- not assignable

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true } }
```
