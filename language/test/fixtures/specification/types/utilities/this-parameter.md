# ThisParameterType

`ThisParameterType` extracts an explicit function receiver.

## cases

### thisparametertype extracts explicit this

> Explicit `this` parameters are returned as their own type.

```ts libs=es5
type Receiver = ThisParameterType<(this: { id: string }, value: number) => void>;

const ok: Receiver = { id: "u1" };
ok.id satisfies string;
```

### thisparametertype rejects unrelated receivers

> Extracted receiver types keep their fields.

```ts libs=es5
type Receiver = ThisParameterType<(this: { id: string }, value: number) => void>;

const bad: Receiver = { name: "Ada" };
```

- contains: excess property

### thisparametertype returns unknown without this

> Functions without explicit receivers produce unknown.

```ts libs=es5
type Receiver = ThisParameterType<(value: number) => void>;

const ok: Receiver = { anything: true };
```
