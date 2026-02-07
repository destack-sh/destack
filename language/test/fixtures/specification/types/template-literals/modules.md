# Template Literal Modules

## modules

### template literal aliases resolve through re export chains

> Template literal aliases should resolve through re-export chains.
> The imported alias should keep its generic substitution behavior.

```ds:base.ds
export type Topic<T extends string> = `topic:${T}`;
```

```ds:forward.ds
export type { Topic } from "./base";
```

```ds:main.ds
import { Topic } from "./forward";

declare const topic: Topic<"orders">;
topic satisfies `topic:${"orders"}`;
```

### template literal aliases resolve through namespace imports

> Template literal aliases should resolve through namespace imports.
> Namespace-qualified references should still bind generic substitutions correctly.

```ds:shared.ds
export type Channel<T extends string> = `channel:${T}`;
```

```ds:main.ds
import * as Shared from "./shared";

declare const value: Shared.Channel<"events">;
value satisfies `channel:${"events"}`;
```

### associated template projections resolve across module boundaries

> Associated template projections should resolve across module boundaries.
> Imported owners must be selected before associated member projection.

```ds:envelope.ds
export interface Envelope<T extends string> {
    type Label<U extends string> = `${T}:${U}`;
}
```

```ds:message.ds
import { Envelope } from "./envelope";

export class Message<T extends string> implements Envelope<T> {}
```

```ds:main.ds
import { Message } from "./message";

declare const label: Message<"orders">.Label<"created">;

// owner symbol must be resolved before projection
label satisfies `${"orders"}:${"created"}`;
```

### mapped template aliases preserve remapped keys across module imports

> Mapped template aliases should preserve remapped keys across module imports.
> The imported alias should retain key remapping and value mapping semantics.

```ds:shape.ds
export type HandlerMap<T> = {
    [K in keyof T as `on-${K}`]: T[K]
};
```

```ds:main.ds
import { HandlerMap } from "./shape";

declare const handlers: HandlerMap<{ ready: boolean }>;
handlers satisfies { "on-ready": boolean };
```
