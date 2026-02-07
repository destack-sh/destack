# Template Literal Associated Interactions

## associated

### class associated aliases can project template literals from owner substitutions

> Class associated aliases can project template literals from owner substitutions.
> The projected member should resolve after binding outer owner parameters.

```ds
class Topic<T extends string> {
    type Channel = `topic:${T}`;
}

declare const channel: Topic<"orders">.Channel;
channel satisfies `topic:${"orders"}`;
```

### interface generic associated defaults can compose template projections

> Interface generic associated defaults can compose template projections.
> Implementors should inherit defaults and preserve owner/member substitutions.

```ds
interface Envelope<T extends string> {
    type Label<U extends string> = `${T}:${U}`;
}

class Message<T extends string> implements Envelope<T> {}

declare const label: Message<"orders">.Label<"created">;
label satisfies `${"orders"}:${"created"}`;
```

### associated aliases can remap keys with template literals

> Associated aliases can include mapped key remapping with template literals.
> Projection through the owner should keep the remapped shape intact.

```ds
interface EventShape<T> {
    type Handlers = {
        [K in keyof T as `on-${K}`]: T[K]
    };
}

class Bus<T> implements EventShape<T> {}

declare const handlers: Bus<{ ready: boolean, message: string }>.Handlers;

// projection should preserve remapped key/value pairs
handlers satisfies { "on-ready": boolean, "on-message": string };
```

### associated aliases can combine conditional template inference and projection

> Associated aliases can combine conditional template inference and projection.
> The projected type should expose the inferred template segment.

```ds
class EventName<T extends string> {
    type Kind = T extends `evt:${infer Name}` ? Name : never;
}

declare const kind: EventName<"evt:login">.Kind;
kind satisfies "login";
```

### constrained owner projections preserve template literal associated aliases

> Associated template aliases should project through constrained generic owners.
> Constraint solving should happen before owner/member projection.

```ds
interface Envelope<T extends string> {
    type Label<U extends string> = `${T}:${U}`;
}

function project<T extends Envelope<"audit">>(value: T.Label<"entry">): T.Label<"entry"> {
    value
}

declare const value: "audit:entry";
project(value) satisfies `${"audit"}:${"entry"}`;
```
