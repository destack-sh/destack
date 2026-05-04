# Interface Declarations

Interface declaration validation.

## rejections

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

## declarations

### named default export interfaces are allowed

> Default export interfaces are accepted when they provide a name.

```ds
export default interface Config {
    value: number;
}
```

### interfaces may extend multiple parents

> Interfaces can extend multiple interface parents.

```ds
interface Named {
    name: string;
}

interface Timestamped {
    updatedAt: int64;
}

interface Entity extends Named, Timestamped {
    id: int64;
}

declare const entity: Entity;
entity.name satisfies string;
entity.updatedAt satisfies int64;
entity.id satisfies int64;
```
