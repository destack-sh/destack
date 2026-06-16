# Exports

Exported bindings may infer types locally.
Other modules use those types without cross-module inference cycles.

## exports

### declared return types cross module boundaries

Exported declarations keep their declared return types across modules.

```ds:api.ds
export type Api = { do: () => string };

export declare function foo(): Api;
```

```ds:module-b.ds
import { foo } from "./api.ds";

export const service = { api: foo() };
```

```ds:main.ds
import { service } from "./module-b.ds";

service.api.do() satisfies string;
```

### function return types cross module boundaries

Exported functions keep return types inferred from their bodies.

```ds:factory.ds
export function make() {
    return { ok: true };
}
```

```ds:main.ds
import { make } from "./factory.ds";

make().ok satisfies boolean;
```

### exported values use local inference

Exported values keep the types inferred in their declaring module.

```ds:config.ds
export const config = { port: 8080, label: "dev" };
export let counter = 0;
export const pinned = 0;
```

```ds:main.ds
import { config } from "./config.ds";
import { counter } from "./config.ds";
import { pinned } from "./config.ds";

config.port satisfies number;
config.label satisfies string;
counter satisfies number;
pinned satisfies 0;
```

## local chains

### generic chains export concrete result types

Exported values keep the type produced by a local generic chain.

```ds:registry-chain.ds
export interface Registry<R> {
    get<K: keyof R>(key: K): R[K];
}

export interface RegistryChain<R> {
    entry<K: string, Value>(key: K, value: Value): RegistryChain<R & { [P in K]: Value }>;

    build(): Registry<R>;
}

export declare function createRegistry(): RegistryChain<{}>;
```

```ds:registry.ds
import { createRegistry } from "./registry-chain.ds";

export const registry = createRegistry()
    .entry("user", { id: 1, name: "Ada" })
    .entry("count", 2)
    .build();
```

```ds:main.ds
import { registry } from "./registry.ds";

registry.get("user") satisfies { id: number; name: string };
registry.get("count") satisfies number;
```

### generic chains export key constraints

Exported values keep key constraints produced by a local generic chain.

```ds:registry-chain.ds
export interface Registry<R> {
    get<K: keyof R>(key: K): R[K];
}

export interface RegistryChain<R> {
    entry<K: string, Value>(key: K, value: Value): RegistryChain<R & { [P in K]: Value }>;

    build(): Registry<R>;
}

export declare function createRegistry(): RegistryChain<{}>;
```

```ds:registry.ds
import { createRegistry } from "./registry-chain.ds";

export const registry = createRegistry().entry("user", { id: 1, name: "Ada" }).build();
```

```ds:main.ds
import { registry } from "./registry.ds";

registry.get("missing");
```

- contains: not assignable

## default exports

### default exports keep inferred return types

Default exports keep inferred return types across modules.

```ds:defaults.ds
export default function make() {
    return { ok: true };
}
```

```ds:main.ds
import make from "./defaults.ds";

make().ok satisfies boolean;
```

## declared imports

### exported values can use declared import signatures

Exported values may depend on declared import signatures.

```ds:builder.ds
export declare const createCounter: () => {
    count: (value: string) => number;
};
```

```ds:counter.ds
import { createCounter } from "./builder.ds";

export const count = createCounter().count("user");
```

```ds:main.ds
import { count } from "./counter.ds";

count satisfies number;
```

## dependencies

### exported values can depend on other inferred exports

Exported values may depend on inferred exports from another module.

```ds:builder.ds
export const createCounter = () => ({
    count: (value: string) => value.length,
});
```

```ds:counter.ds
import { createCounter } from "./builder.ds";

export const count = createCounter().count("user");
```

```ds:main.ds
import { count } from "./counter.ds";

count satisfies number;
```

### exported aliases keep imported shapes

Exported aliases keep imported value shapes.

```ds:values.ds
export const config = { nested: { ok: true } };
```

```ds:module-b.ds
import { config } from "./values.ds";

export const shared = config;
```

```ds:main.ds
import { shared } from "./module-b.ds";

shared.nested.ok satisfies boolean;
```

### exported wrappers keep imported shapes

Exported wrappers keep imported value shapes.

```ds:values.ds
export const config = { nested: { ok: true } };
```

```ds:module-b.ds
import { config } from "./values.ds";

export const shared = { config, extra: true };
```

```ds:main.ds
import { shared } from "./module-b.ds";

shared.config.nested.ok satisfies boolean;
shared.extra satisfies boolean;
```
