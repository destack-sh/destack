# OmitThisParameter

`OmitThisParameter` removes an explicit function receiver.

## cases

### omitthisparameter removes explicit this

> Explicit receivers are removed from the callable type.

```ts libs=es5
type Fn = OmitThisParameter<(this: { id: string }, value: number) => string>;

const fn: Fn = value => String(value);
fn(1) satisfies string;
```

### omitthisparameter keeps argument types

> Non-receiver parameters remain checked.

```ts libs=es5
type Fn = OmitThisParameter<(this: { id: string }, value: number) => string>;

const fn: Fn = value => String(value);
fn("bad");
```

- contains: not assignable

### omitthisparameter keeps functions without this

> Functions without explicit receivers are unchanged.

```ts libs=es5
type Fn = OmitThisParameter<(value: number) => string>;

const fn: Fn = value => String(value);
fn(1) satisfies string;
```
