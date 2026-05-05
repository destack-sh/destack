# ThisParameterType

`ThisParameterType` extracts an explicit function receiver.

### ThisParameterType extracts explicit this

```ds
type Receiver = ThisParameterType<(this: { id: string }, value: number) => void>;

const ok: Receiver = { id: "u1" };
ok.id satisfies string;
```

### ThisParameterType rejects unrelated receivers

```ds
type Receiver = ThisParameterType<(this: { id: string }, value: number) => void>;

const bad: Receiver = { name: "Ada" };
```

- contains: excess property

### ThisParameterType returns unknown without this

```ds
type Receiver = ThisParameterType<(value: number) => void>;

const ok: Receiver = { anything: true };
```
