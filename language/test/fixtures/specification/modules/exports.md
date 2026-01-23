# Exports

Exported bindings may infer types locally, while other modules use those
types without cross module inference cycles.

## Basic Exports

### export inference tracks declared return types

> Export inference uses declared return types across modules.

```ts:api.ts
export type Api = { do: () => string };

export declare function foo(): Api;
```

```ts:module-b.ts
import { foo } from "./api";

export const service = { api: foo() };
```

```ts:consumer.ts
import { service } from "./module-b";

service.api.do() satisfies string;
```

### export inference tracks inferred exports

> Export inference preserves local inference for exported values.

```ts:config.ts
export const config = { port: 8080, label: "dev" };
export let counter = 0;
export const pinned = 0;
```

```ts:consumer.ts
import { config } from "./config";
import { counter } from "./config";
import { pinned } from "./config";

config.port satisfies number;
config.label satisfies string;
counter satisfies number;
pinned satisfies 0;
```

## Fluent Builders

### export inference infers fluent builder chains

> Export inference infers fluent builder chains within a module.

```ts:server-builder.ts
export interface Server<R> {
    invoke<K extends keyof R>(path: K): R[K];
}

export interface ServerBuilder<R> {
    query<P extends string, Result>(
        path: P,
        handler: () => Result
    ): ServerBuilder<R & { [K in P]: Result }>;

    build(): Server<R>;
}

export declare function createServer(): ServerBuilder<{}>;
```

```ts:server.ts
import { createServer } from "./server-builder";

export const server = createServer()
    .query("/user/get", () => ({ id: 1, name: "Ada" }))
    .query("/user/list", () => ({ count: 2 }))
    .build();
```

```ts:consumer.ts
import { server } from "./server";

server.invoke("/user/get") satisfies { id: number, name: string };
server.invoke("/user/list") satisfies { count: number };
```

### export inference rejects unknown routes

> Export inference rejects unknown builder routes.

```ts:server-builder.ts
export interface Server<R> {
    invoke<K extends keyof R>(path: K): R[K];
}

export interface ServerBuilder<R> {
    query<P extends string, Result>(
        path: P,
        handler: () => Result
    ): ServerBuilder<R & { [K in P]: Result }>;

    build(): Server<R>;
}

export declare function createServer(): ServerBuilder<{}>;
```

```ts:server.ts
import { createServer } from "./server-builder";

export const server = createServer()
    .query("/user/get", () => ({ id: 1, name: "Ada" }))
    .build();
```

```ts:consumer.ts
import { server } from "./server";

server.invoke("/missing");
```

- contains: not assignable

## Re-exports

### export inference preserves reexports

> Re-export chains preserve export inference.

```ts:a.ts
export const value = { ok: true };
```

```ts:b.ts
export { value } from "./a";
```

```ts:c.ts
import { value } from "./b";

value.ok satisfies boolean;
```

### export inference preserves renamed reexports

> Renamed reexports preserve export inference.

```ts:a.ts
export const value = { ok: true };
```

```ts:b.ts
export { value as renamed } from "./a";
```

```ts:c.ts
import { renamed } from "./b";

renamed.ok satisfies boolean;
```

### export inference preserves export star

> Export star preserves export inference.

```ts:a.ts
export const value = { ok: true };
```

```ts:b.ts
export * from "./a";
```

```ts:c.ts
import { value } from "./b";

value.ok satisfies boolean;
```

### export inference preserves namespace reexports

> Export namespace preserves export inference.

```ts:a.ts
export const value = { ok: true };
```

```ts:b.ts
export * as ns from "./a";
```

```ts:c.ts
import { ns } from "./b";

ns.value.ok satisfies boolean;
```

### export inference preserves default reexports

> Default reexports preserve export inference.

```ts:defaults.ts
export default function make() {
    return { ok: true };
}
```

```ts:reexport.ts
export { default as make } from "./defaults";
```

```ts:consumer.ts
import { make } from "./reexport";

make().ok satisfies boolean;
```

## Default Exports

### export inference tracks default exports

> Export inference tracks default exports across modules.

```ts:defaults.ts
export default function make() {
    return { ok: true };
}
```

```ts:consumer.ts
import make from "./defaults";

make().ok satisfies boolean;
```

## Namespace Imports

### export inference tracks namespace imports

> Export inference tracks namespace imported values.

```ts:exports.ts
export const value = { ok: true };
```

```ts:module-b.ts
import * as mod from "./exports";

export const forwarded = mod.value;
```

```ts:consumer.ts
import { forwarded } from "./module-b";

forwarded.ok satisfies boolean;
```

## Declared Imports

### export inference uses declared imports

> Export inference can depend on declared imports.

```ts:builder.d.ts
export declare const createServer: () => {
    route: (path: string) => number
};
```

```ts:server.ts
import { createServer } from "./builder";

export const server = createServer().route("/user");
```

```ts:consumer.ts
import { server } from "./server";

server satisfies number;
```

## Export Dependencies

### export inference uses exported imports

> Export inference may depend on other module export inference.

```ts:builder.ts
export const createServer = () => ({
    route: (path: string) => path.length,
});
```

```ts:server.ts
import { createServer } from "./builder";

export const server = createServer().route("/user");
```

```ts:consumer.ts
import { server } from "./server";

server satisfies number;
```

## Type-only Exports

### export inference preserves type-only reexports

> Export inference preserves type-only reexports.

```ts:types.ts
export type Options = { strict: boolean };
```

```ts:module-b.ts
export { type Options } from "./types";
```

```ts:consumer.ts
import type { Options } from "./module-b";

const options: Options = { strict: true };
options.strict satisfies boolean;
```

### export inference rejects type-only values

> Type-only exports do not provide runtime values.

```ts:types.ts
export type Options = { strict: boolean };
```

```ts:module-b.ts
export { type Options } from "./types";
```

```ts:consumer.ts
import { Options } from "./module-b";

Options;
```

- contains: type only

## Export Circularity

### export inference rejects inference cycles

> Export inference cycles require explicit annotations.

```ts:a.ts
import { y } from "./b";

export const x = y;
```

```ts:b.ts
import { x } from "./a";

export const y = x;
```

- contains: annotation

### export inference allows partially annotated cycles

> Export inference allows cycles that are broken by explicit annotations.

```ts:a.ts
import { y } from "./b";

export const x: number = y;
```

```ts:b.ts
import { x } from "./a";

export const y = x;
```

### export inference allows annotated cycles

> Annotated exports can break inference cycles.

```ts:a.ts
import { y } from "./b";

export const x: number = y;
```

```ts:b.ts
import { x } from "./a";

export const y: number = x;
```
