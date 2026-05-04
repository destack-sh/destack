# Callbacks

Callbacks receive contextual parameter types from their call site.

## context

### contextual parameters flow into callbacks

> Callback parameters use the declared parameter type.

```ts
declare function drive(callback: (value: string) => void): void;

drive(value => {
    value satisfies string;
});
```

### nested callbacks preserve contextual parameter types

> Nested callback calls keep contextual parameter types.

```ts
declare function drive<T>(value: T, callback: (value: T) => void): void;
declare function wrap(callback: (value: string) => string): string;

const result = wrap(value => {
    drive(value, current => {
        current satisfies string;
    });

    return value;
});

result satisfies string;
```

## overloads

### contextual callbacks select overloads in declaration order

> Contextual callback typing uses overload declaration order.

```ts
declare function choose(value: string): "string";
declare function choose(value: number): "number";
declare function drive(callback: (value: string) => "string"): "ok";

const result = drive(value => choose(value));
result satisfies "ok";
```

### contextual callbacks do not select later overloads

> Later overload branches do not replace an earlier contextual match.

```ts
declare function choose(value: string): "string";
declare function choose(value: number): "number";
declare function drive(callback: (value: string) => "string"): "ok";

const result = drive(value => choose(value));
result satisfies "number";
```

- expected "number", found "ok"

## widening

### mutable callback inputs widen

> Mutable callback inputs do not retain literal precision.

```ts
declare function drive<T>(value: T, callback: (value: T) => void): void;

let input = "ready";

drive(input, value => {
    value satisfies "ready";
});
```

- expected "ready", found string

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true } }
```
