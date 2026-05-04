# Exports

Exported bindings may infer types locally.
Other modules use those types without cross-module inference cycles.

## exports

### declared return types cross module boundaries

Exported declarations keep their declared return types across modules.

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

### function return types cross module boundaries

Exported functions keep return types inferred from their bodies.

```ts:factory.ts
export function make() {
    return { ok: true };
}
```

```ts:main.ts
import { make } from "./factory";

make().ok satisfies boolean;
```


### exported values use local inference

Exported values keep the types inferred in their declaring module.

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



## local chains

### generic chains export concrete result types

> Exported values keep the type produced by a local generic chain.

```ts:registry-chain.ts
export interface Registry<R> {
    get<K extends keyof R>(key: K): R[K];
}

export interface RegistryChain<R> {
    entry<K extends string, Value>(
        key: K,
        value: Value
    ): RegistryChain<R & { [P in K]: Value }>;

    build(): Registry<R>;
}

export declare function createRegistry(): RegistryChain<{}>;
```

```ts:registry.ts
import { createRegistry } from "./registry-chain";

export const registry = createRegistry()
    .entry("user", { id: 1, name: "Ada" })
    .entry("count", 2)
    .build();
```

```ts:main.ts
import { registry } from "./registry";

registry.get("user") satisfies { id: number, name: string };
registry.get("count") satisfies number;
```

### generic chains export key constraints

> Exported values keep key constraints produced by a local generic chain.

```ts:registry-chain.ts
export interface Registry<R> {
    get<K extends keyof R>(key: K): R[K];
}

export interface RegistryChain<R> {
    entry<K extends string, Value>(
        key: K,
        value: Value
    ): RegistryChain<R & { [P in K]: Value }>;

    build(): Registry<R>;
}

export declare function createRegistry(): RegistryChain<{}>;
```

```ts:registry.ts
import { createRegistry } from "./registry-chain";

export const registry = createRegistry()
    .entry("user", { id: 1, name: "Ada" })
    .build();
```

```ts:main.ts
import { registry } from "./registry";

registry.get("missing");
```

- contains: not assignable


## default exports

### default exports keep inferred return types

Default exports keep inferred return types across modules.

```ts:defaults.ts
export default function make() {
    return { ok: true };
}
```

```ts:main.ts
import make from "./defaults";

make().ok satisfies boolean;
```


## declared imports

### exported values can use declared import signatures

Exported values may depend on declared import signatures.

```ts:builder.ts
export declare const createCounter: () => {
    count: (value: string) => number
};
```

```ts:counter.ts
import { createCounter } from "./builder";

export const count = createCounter().count("user");
```

```ts:main.ts
import { count } from "./counter";

count satisfies number;
```


## dependencies

### exported values can depend on other inferred exports

Exported values may depend on inferred exports from another module.

```ts:builder.ts libs=es5
export const createCounter = () => ({
    count: (value: string) => value.length,
});
```

```ts:counter.ts
import { createCounter } from "./builder";

export const count = createCounter().count("user");
```

```ts:main.ts
import { count } from "./counter";

count satisfies number;
```

### exported aliases keep imported shapes

Exported aliases keep imported value shapes.

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

### exported wrappers keep imported shapes

Exported wrappers keep imported value shapes.

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
