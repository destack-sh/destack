# ThisParameterType

`ThisParameterType` extracts an explicit function receiver.

## receivers

### ThisParameterType extracts explicit this

The declared receiver type comes out.

```ds
type Receiver = ThisParameterType<(this: { id: string }, value: number) => void>;

const ok: Receiver = { id: "u1" };
ok.id satisfies string;
```

### ThisParameterType rejects unrelated receivers

The extracted shape is exact.

```ds
type Receiver = ThisParameterType<(this: { id: string }, value: number) => void>;

const bad: Receiver = { name: "Ada" };
```

- contains: excess property

### ThisParameterType returns unknown without this

No declared receiver means no information.

```ds
type Receiver = ThisParameterType<(value: number) => void>;

const ok: Receiver = { anything: true };
```
