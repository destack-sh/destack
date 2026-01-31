# Interface Declarations

Tests for interface declaration validation.

## invalid declarations

### abstract interfaces are rejected

> Interfaces cannot be abstract.

```ds
abstract interface Config {}
```

- contains: invalid interface

### default export interfaces must be named

> Default export interfaces require a name.

```ds
export default interface {
    value: number;
}
```

- contains: invalid interface

### empty extends clauses are rejected

> Interfaces cannot declare empty extends clauses.

```ds
interface Config extends {
}
```

- contains: invalid lineage
