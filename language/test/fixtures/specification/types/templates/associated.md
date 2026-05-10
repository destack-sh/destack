# Template Associated Types

## associated

### associated aliases can use outer generic arguments

Associated aliases can refer to generic parameters from their type.

```ds
class Topic<T: string> {
    type Channel = `topic:${T}`;
}

declare const channel: Topic<"orders">.Channel;
channel satisfies `topic:${"orders"}`;
```

### interface defaults can use outer and member parameters

Implementors inherit associated defaults that mention both parameter lists.

```ds
interface Envelope<T: string> {
    type Label<U: string> = `${T}:${U}`;
}

class Message<T: string> implements Envelope<T> {}

declare const label: Message<"orders">.Label<"created">;
label satisfies `${"orders"}:${"created"}`;
```

### associated aliases can remap keys

Associated aliases can include mapped key remapping with template literals.
The resulting object keeps the remapped keys and value types.

```ds
interface EventShape<T> {
    type Handlers = {
        [K in keyof T as `on-${K}`]: T[K];
    };
}

class Bus<T> implements EventShape<T> {}

declare const handlers: Bus<{ ready: boolean; message: string }>.Handlers;

handlers satisfies { "on-ready": boolean; "on-message": string };
```

### associated aliases can infer template spans

Conditional aliases can infer spans from template literals.

```ds
class EventName<T: string> {
    type Kind = T extends `evt:${infer Name}` ? Name : never;
}

declare const kind: EventName<"evt:login">.Kind;
kind satisfies "login";
```

### constrained generics can use associated aliases

Generic constraints make associated aliases available in parameter types.

```ds
interface Envelope<T: string> {
    type Label<U: string> = `${T}:${U}`;
}

function label<T: Envelope<"audit">>(value: T.Label<"entry">): T.Label<"entry"> {
    value
}

declare const value: "audit:entry";
label(value) satisfies `${"audit"}:${"entry"}`;
```
