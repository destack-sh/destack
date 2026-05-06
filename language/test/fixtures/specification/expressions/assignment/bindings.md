# Bindings

## definite assignment

### reads require prior assignment

A declared local must be assigned before it is read.

```ds
let value: string;
value satisfies string;
```

- contains: definitely assigned

### writes make locals readable

A declared local becomes readable after assignment.

```ds
let value: string;
value = "ready";
value satisfies string;
```

### writes check declared types

Assignment checks the declared type.

```ds
let value: string;
value = 1;
```

- contains: not assignable
