# OmitThisParameter

`OmitThisParameter` removes an explicit function receiver.

## receivers

### OmitThisParameter removes explicit this

```ds
type Fn = OmitThisParameter<(this: { id: string }, value: number) => string>;

const fn: Fn = value => String(value);
fn(1) satisfies string;
```

### OmitThisParameter keeps argument types

```ds
type Fn = OmitThisParameter<(this: { id: string }, value: number) => string>;

const fn: Fn = value => String(value);
fn("bad");
```

- contains: not assignable

### OmitThisParameter keeps functions without this

```ds
type Fn = OmitThisParameter<(value: number) => string>;

const fn: Fn = value => String(value);
fn(1) satisfies string;
```
