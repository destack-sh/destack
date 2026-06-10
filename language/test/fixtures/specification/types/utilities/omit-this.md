# OmitThisParameter

`OmitThisParameter` removes an explicit function receiver.

## receivers

### OmitThisParameter removes explicit this

The receiver requirement disappears.

```ds
type Fn = OmitThisParameter<(this: { id: string }, value: number) => string>;

const fn: Fn = (value) => String(value);
fn(1) satisfies string;
```

### OmitThisParameter keeps argument types

Only the receiver changes.

```ds
type Fn = OmitThisParameter<(this: { id: string }, value: number) => string>;

const fn: Fn = (value) => String(value);
fn("bad");
```

- contains: not assignable

### OmitThisParameter keeps functions without this

Nothing to remove, nothing changes.

```ds
type Fn = OmitThisParameter<(value: number) => string>;

const fn: Fn = (value) => String(value);
fn(1) satisfies string;
```
