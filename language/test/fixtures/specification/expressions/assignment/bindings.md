# Bindings

## definite assignment

### let bindings allow definite assignment assertions

> Definite assignment assertions allow a binding to be written before first use.

```ds
let value!: string;
value = "ready";
value satisfies string;
```

### definite assignment assertions require writes before reads

> A definite assignment assertion does not make an unwritten binding readable.

```ds
let value!: string;
value satisfies string;
```

- contains: definitely assigned

### definite assignment assertions still enforce declared types

> Definite assignment assertions do not weaken declared type rules.

```ds
let value!: string;
value = 1;
```

- type 1 is not assignable to type string
