# Bindings

Bindings must be definitely assigned before use.

## definite assignment

### reads require prior assignment

A declared local must be assigned before it is read.

```ds
let value: string;
value satisfies string;
```

- contains: definitely assigned

### branch assignment requires every path

A branch assignment makes a local readable only when every normally completed branch assigns it.

```ds
declare const condition: boolean;

let value: string;
if (condition) {
    value = "ready";
}

value satisfies string;
```

- contains: definitely assigned

### undefined models optional state

Maybe-present state must be written as an initialized union.

```ds
declare const condition: boolean;

let value: string | undefined = undefined;
if (condition) {
    value = "ready";
}

value satisfies string | undefined;
```

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
