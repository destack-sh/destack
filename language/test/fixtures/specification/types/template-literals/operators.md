# Template Literal Operators

## operators

### mapped key remapping can use template literal keys

> Mapped key remapping can synthesize keys with template literals.
> The remapped shape should preserve source field value types.

```ds
type HandlerMap<T> = {
    [K in keyof T as `on-${K}`]: T[K]
};

interface Events {
    ready: boolean
    message: string
}

const handlers: HandlerMap<Events> = {
    "on-ready": true,
    "on-message": "ok",
};
```

### mapped key remapping rejects incompatible remapped values

> Remapped template keys still enforce mapped value types.
> The remapped field should still fail with a value compatibility error.

```ds
type HandlerMap<T> = {
    [K in keyof T as `on-${K}`]: T[K]
};

interface Events {
    ready: boolean
}

const handlers: HandlerMap<Events> = {
    "on-ready": "bad",
};
```

- contains: not assignable

### indexed access can target remapped template literal keys

> Indexed access should work over remapped template-literal keys.
> The indexed result should match the projected mapped field value type.

```ds
type HandlerMap<T> = {
    [K in keyof T as `on-${K}`]: T[K]
};

type Value = HandlerMap<{ name: string }>["on-name"];

declare const value: Value;
value satisfies string;
```

### conditional template inference composes with mapped output

> Template inference in conditionals should compose with mapped output types.
> Each projected field should carry the inferred payload literal.

```ds
type PayloadName<E> = E extends `evt:${infer Name}` ? Name : never;

type PayloadMap<E extends string> = {
    [K in E as `payload:${K}`]: PayloadName<K>
};

type Names = PayloadMap<"evt:user" | "evt:order">;

declare const user: Names["payload:evt:user"];
declare const order: Names["payload:evt:order"];

// each field must carry the inferred payload segment
user satisfies "user";
order satisfies "order";
```

### satisfies keeps template literal constraints with remapped keys

> `satisfies` should validate remapped template-keyed object literals without widening away constraints.
> The check should enforce key spelling and value compatibility in one pass.

```ds
type HandlerMap<T> = {
    [K in keyof T as `on-${K}`]: T[K]
};

const handlers = {
    "on-open": true,
    "on-close": false,
} satisfies HandlerMap<{ open: boolean, close: boolean }>;

handlers["on-open"] satisfies boolean;
```
