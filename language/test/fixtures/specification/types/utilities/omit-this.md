# OmitThisParameter

`OmitThisParameter` removes an explicit function receiver.

### OmitThisParameter removes explicit this

```ts libs=es5
type Fn = OmitThisParameter<(this: { id: string }, value: number) => string>;

const fn: Fn = value => String(value);
fn(1) satisfies string;
```

### OmitThisParameter keeps argument types

```ts libs=es5
type Fn = OmitThisParameter<(this: { id: string }, value: number) => string>;

const fn: Fn = value => String(value);
fn("bad");
```

- contains: not assignable

### OmitThisParameter keeps functions without this

```ts libs=es5
type Fn = OmitThisParameter<(value: number) => string>;

const fn: Fn = value => String(value);
fn(1) satisfies string;
```
