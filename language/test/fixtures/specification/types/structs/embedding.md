# Struct Embedding

## embedded fields

### struct embedding exposes fields

> Embedded structs contribute their fields to the outer struct.

```ds
struct Transform {
    x: int32
}

struct Entity {
    ...Transform
}

const entity = Entity { x: 1 };
entity.x satisfies int32;
```

### struct embedding exposes fields from multiple embeds

> Multiple embeds contribute fields to the outer struct.

```ds
struct Position {
    x: int32
}

struct Velocity {
    y: int32
}

struct Transform {
    ...Position
    ...Velocity
}

const transform = Transform { x: 1, y: 2 };
transform.x satisfies int32;
transform.y satisfies int32;
```

### struct embedding chains through nested structs

> Nested embeds surface fields from embedded parents.

```ds
struct Base {
    id: int32
}

struct Middle {
    ...Base
    name: string
}

struct Outer {
    ...Middle
    active: boolean
}

const outer = Outer { id: 1, name: "ok", active: true };
outer.id satisfies int32;
outer.name satisfies string;
outer.active satisfies boolean;
```
