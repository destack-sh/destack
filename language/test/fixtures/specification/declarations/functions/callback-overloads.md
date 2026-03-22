# Callback Overloads

## contextual routing and ordering

### contextual callbacks pick declaration ordered overloads

Contextual callback typing should choose the first declaration-ordered overload that matches the callback shape.

```ts
declare function choose(value: string): "string";
declare function choose(value: number): "number";
declare function drive(callback: (value: string) => "string"): "ok";

const result = drive(value => choose(value));
result satisfies "ok";
```

### contextual callbacks do not select later overload results

Once an earlier overload matches contextually, later overload branches must not change the inferred result.

```ts
declare function choose(value: string): "string";
declare function choose(value: number): "number";
declare function drive(callback: (value: string) => "string"): "ok";

const result = drive(value => choose(value));
result satisfies "number";
```

- expected "ready", found string (not assignable)

### nested callback wrappers preserve contextual parameter typing

Contextual parameter types should flow through wrapper callbacks without losing argument precision.

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

### mutable callback inputs do not preserve literal contextual precision

Passing mutable callback inputs should widen literals, so contextual typing does not retain const-level precision.

```ts
declare function drive<T>(value: T, callback: (value: T) => void): void;

let input = "ready";

drive(input, value => {
    value satisfies "ready";
});
```

- expected "ready", found string (not assignable)

```json:destack.json
{ "compilerOptions": { "allowTs": true, "checkTs": true } }
```