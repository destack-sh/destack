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

```ts:main.ts
import { service } from "./module-b";

service.api.do() satisfies string;
```

### export inference tracks inferred function returns

> Export inference tracks function return types inferred from bodies.

```ts:factory.ts
export function make() {
    return { ok: true };
}
```

```ts:main.ts
import { make } from "./factory";

make().ok satisfies boolean;
```


### export inference tracks inferred exports

> Export inference preserves local inference for exported values.

```ts:config.ts
export const config = { port: 8080, label: "dev" };
export let counter = 0;
export const pinned = 0;
```

```ts:main.ts
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

```ts:main.ts
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

```ts:main.ts
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

```ts:main.ts
import { value } from "./b";

value.ok satisfies boolean;
```

### export inference preserves multi-hop reexports

> Multi-hop reexports preserve export inference.

```ts:a.ts
export const value = { ok: true };
```

```ts:b.ts
export { value } from "./a";
```

```ts:c.ts
export { value } from "./b";
```

```ts:main.ts
import { value } from "./c";

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

```ts:main.ts
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

```ts:main.ts
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

```ts:main.ts
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

```ts:main.ts
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

```ts:main.ts
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

```ts:main.ts
import { forwarded } from "./module-b";

forwarded.ok satisfies boolean;
```

## Declared Imports

### export inference uses declared import signatures

> Export inference can depend on declared imports.

```ts:builder.ts
export declare const createRouter: () => {
    route: (path: string) => number
};
```

```ts:server.ts
import { createRouter } from "./builder";

export const routeLength = createRouter().route("/user");
```

```ts:main.ts
import { routeLength } from "./server";

routeLength satisfies number;
```

## Export Dependencies

### export inference follows inferred exports

> Export inference may depend on other module export inference.

```ts:builder.ts libs=es5
export const createRouter = () => ({
    route: (path: string) => path.length,
});
```

```ts:server.ts
import { createRouter } from "./builder";

export const routeLength = createRouter().route("/user");
```

```ts:main.ts
import { routeLength } from "./server";

routeLength satisfies number;
```

### export inference forwards imported values

> Export inference preserves shapes from imported values.

```ts:values.ts
export const config = { nested: { ok: true } };
```

```ts:module-b.ts
import { config } from "./values";

export const shared = config;
```

```ts:main.ts
import { shared } from "./module-b";

shared.nested.ok satisfies boolean;
```

### export inference forwards wrapped imports

> Export inference preserves shapes when imported values are wrapped.

```ts:values.ts
export const config = { nested: { ok: true } };
```

```ts:module-b.ts
import { config } from "./values";

export const shared = { config, extra: true };
```

```ts:main.ts
import { shared } from "./module-b";

shared.config.nested.ok satisfies boolean;
shared.extra satisfies boolean;
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

```ts:main.ts
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

```ts:main.ts
import { Options } from "./module-b";

const value = Options;
```

- contains: type only

## Type-only Imports

### export inference rejects type-only value usage

> Type-only imports do not provide runtime values.

```ts:types.ts
export type Options = { strict: boolean };
```

```ts:module-b.ts
import type { Options } from "./types";

export const value = Options;
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
