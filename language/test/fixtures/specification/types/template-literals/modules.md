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

### template literal conditionals stay precise through renamed re-exports

> Conditional template aliases should preserve branch precision through renamed re-export paths.

```ds:base.ds
export type Segment<T extends string> = T extends `id:${infer S}` ? S : never;
```

```ds:index.ds
export type { Segment as ParseSegment } from "./base";
```

```ds:main.ds
import { ParseSegment } from "./index";

declare const segment: ParseSegment<"id:users">;
segment satisfies "users";
```

### namespace imports preserve template conditional false branches

> Namespace-qualified template conditionals should preserve false-branch semantics.

```ds:base.ds
export type Segment<T extends string> = T extends `id:${infer S}` ? S : never;
```

```ds:main.ds
import * as api from "./base";

declare const segment: api.Segment<"users">;
segment satisfies never;
```

### template aliases remain stable across barrel cycles

> Type-only barrel cycles should keep template alias projections stable for consumers.

```ds:a.ds
import type { Topic } from "./b";

export type Label<T extends string> = `label:${T}`;
export type Rebound = Topic<"orders">;
```

```ds:b.ds
export type { Label as Topic } from "./a";
```

```ds:main.ds
import { Label } from "./a";

declare const label: Label<"orders">;
label satisfies `label:${"orders"}`;
```

### conditional template aliases distribute unions through export-star barrels

> Export-star barrels should preserve distributive conditional template alias behavior.

```ds:base.ds
export type Segment<T extends string> = T extends `id:${infer S}` ? S : never;
```

```ds:index.ds
export * from "./base";
```

```ds:main.ds
import { Segment } from "./index";

declare const segment: Segment<"id:users" | "id:posts">;
segment satisfies "users" | "posts";
```

### namespace imports preserve conditional template alias rejection for non matches

> Namespace imports should preserve conditional template false branches for non matching literals.

```ds:base.ds
export type Segment<T extends string> = T extends `id:${infer S}` ? S : never;
```

```ds:main.ds
import * as api from "./base";

declare const segment: api.Segment<"users">;
segment satisfies "users";
```

- contains: not assignable

### associated template projections remain precise through renamed re-exports

> Renamed re-exports should preserve associated template projection substitutions.

```ds:envelope.ds
export interface Envelope<T extends string> {
    type Label<U extends string> = `${T}:${U}`;
}
```

```ds:message.ds
import { Envelope } from "./envelope";

export class Message<T extends string> implements Envelope<T> {}
```

```ds:index.ds
export { Message as PacketMessage } from "./message";
```

```ds:main.ds
import { PacketMessage } from "./index";

declare const label: PacketMessage<"orders">.Label<"created">;
label satisfies `${"orders"}:${"created"}`;
```

### associated template mapped conditionals preserve precision through export-star barrels

> Export-star barrels should preserve associated template mapped conditional substitutions.
> Owner substitution should flow into template spans before mapped conditional evaluation.

```ds:contract.ds
export interface EventShape<Row extends string> {
    type Id = `id:${Row}`;
    type Payload<Flags> = {
        [K in keyof Flags]: Flags[K] extends true ? this.Id : never
    };
}
```

```ds:owner.ds
import type { EventShape } from "./contract";

export class UserEvent implements EventShape<"users"> {}
```

```ds:index.ds
export * from "./owner";
```

```ds:main.ds
import { UserEvent } from "./index";

declare const payload: UserEvent.Payload<{ primary: true, secondary: false }>;
payload satisfies { primary: "id:users", secondary: never };
```

### associated template mapped conditionals preserve precision through namespace imports

> Namespace imports should preserve associated template mapped conditional substitutions.
> Namespace-qualified owners should evaluate the same projection result as direct imports.

```ds:contract.ds
export interface EventShape<Row extends string> {
    type Id = `id:${Row}`;
    type Payload<Flags> = {
        [K in keyof Flags]: Flags[K] extends true ? this.Id : never
    };
}
```

```ds:owner.ds
import type { EventShape } from "./contract";

export class UserEvent implements EventShape<"users"> {}
```

```ds:main.ds
import * as api from "./owner";

declare const payload: api.UserEvent.Payload<{ primary: true, secondary: false }>;
payload satisfies { primary: "id:users", secondary: never };
```
