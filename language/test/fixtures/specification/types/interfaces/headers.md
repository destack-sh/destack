# Interface Headers

Interface headers define names, exports, and parent interfaces.

## modifiers

### interfaces cannot be abstract

> Interfaces cannot be abstract.

```ds
abstract interface Config {}
```

- contains: invalid interface

## exports

### default export interfaces must be named

> Default export interfaces require a name.

```ds
export default interface {
    value: number;
}
```

- contains: invalid interface

### named default export interfaces are allowed

> Default export interfaces are accepted when they provide a name.

```ds
export default interface Config {
    value: number;
}
```

## extends

### empty extends clauses are rejected

> Interfaces cannot declare empty extends clauses.

```ds
interface Config extends {
}
```

- contains: invalid lineage

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
