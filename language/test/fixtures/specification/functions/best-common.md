# Contextual Typing and Best Common Type

Widening should happen at commitment points rather than during type level evaluation.

## Array and tuple literals

### arrays widen element literals in let bindings

> Let bindings should commit array element literals to widened element types.

```ds
let values = [1, 2];

values[0] satisfies number;
```

### arrays do not preserve literal element types in let bindings

> Let bindings should not retain literal element unions for array elements.

```ds
let values = [1, 2];

let union: 1 | 2 = values[0];
```

- contains: not assignable

### const arrays still widen elements without const assertions

> Const bindings alone should not preserve literal element types.

```ds
const values = [1, 2];

values[0] satisfies number;
```

### const arrays do not preserve literal element types without const assertions

> Const bindings alone should not retain literal element unions for arrays.

```ds
const values = [1, 2];

let union: 1 | 2 = values[0];
```

- contains: not assignable

### contextual arrays preserve union element types

> Contextual element types should prevent widening beyond the context.

```ds
const values: (1 | 2)[] = [1, 2];

values[0] satisfies 1 | 2;
```

### contextual arrays do not narrow to a single literal

> Contextual array element unions should not narrow to a single literal.

```ds
const values: (1 | 2)[] = [1, 2];

values[0] satisfies 1;
```

- contains: expected 1

## Conditional expressions

### let conditionals commit to widened types

> Let bindings should widen conditional literal unions when no context constrains them.

```ds
let value = true ? 1 : 2;

value satisfies number;
```

### let conditionals do not preserve literal unions

> Let conditional results should not retain literal unions without context.

```ds
let value = true ? 1 : 2;

let union: 1 | 2 = value;
```

- contains: not assignable

### const conditionals preserve literal unions

> Const bindings should retain literal unions for conditional expressions.

```ds
const value = true ? 1 : 2;

value satisfies 1 | 2;
```

## Contextual object literals

### annotations constrain object literal fields

> Annotations should contextualize object literal fields without changing the annotation.

```ds
type Mode = "dev" | "prod";

const config: { mode: Mode } = { mode: "dev" };

config.mode satisfies Mode;
```

### annotations do not narrow object literal fields to a single literal

> Annotations should not narrow object literal fields to a specific literal.

```ds
type Mode = "dev" | "prod";

const config: { mode: Mode } = { mode: "dev" };

config.mode satisfies "dev";
```

- contains: expected "dev"
