# Template Remapping

## keys

### mapped keys can use templates

Key remapping can use template literal keys while preserving each field type.

```ds
type HandlerMap<T> = {
    [K in keyof T as `on-${K}`]: T[K];
};

interface Events {
    ready: boolean;
    message: string;
}

const handlers: HandlerMap<Events> = {
    "on-ready": true,
    "on-message": "ok",
};
```

### mapped keys keep value types

A remapped key still uses the original field type.

```ds
type HandlerMap<T> = {
    [K in keyof T as `on-${K}`]: T[K];
};

interface Events {
    ready: boolean;
}

const handlers: HandlerMap<Events> = {
    "on-ready": "bad",
};
```

- contains: not assignable

### indexed access can use generated keys

A generated key can be used in indexed access.
The result is the original field type.

```ds
type HandlerMap<T> = {
    [K in keyof T as `on-${K}`]: T[K];
};

type Value = HandlerMap<type { name: string }>["on-name"];

declare const value: Value;
value satisfies string;
```

### remapped keys can use inferred parts

A remapped key can use a template part inferred from the original key.

```ds
type PayloadName<E> = E extends `evt:${infer Name}` ? Name : never;

type PayloadMap<E: string> = {
    [K in E as `payload:${K}`]: PayloadName<K>;
};

type Names = PayloadMap<"evt:user" | "evt:order">;

declare const user: Names["payload:evt:user"];
declare const order: Names["payload:evt:order"];

user satisfies "user";
order satisfies "order";
```

### satisfies checks remapped keys

`satisfies` checks remapped key spelling and value types.

```ds
type HandlerMap<T> = {
    [K in keyof T as `on-${K}`]: T[K];
};

const handlers = {
    "on-open": true,
    "on-close": false,
} satisfies HandlerMap<type { open: boolean; close: boolean }>;

handlers["on-open"] satisfies boolean;
```
