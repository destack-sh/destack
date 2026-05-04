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

> Nested embeds expose fields from embedded parents.

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

### struct embedding supports deeper chains

> Deep embed chains continue to expose embedded fields.

```ds
struct Root {
    id: int32
}

struct Branch {
    ...Root
    label: string
}

struct Trunk {
    ...Branch
    active: boolean
}

struct Canopy {
    ...Trunk
    count: int32
}

const canopy = Canopy { id: 1, label: "ok", active: true, count: 2 };
canopy.id satisfies int32;
canopy.label satisfies string;
canopy.active satisfies boolean;
canopy.count satisfies int32;
```

### struct embedding rejects missing required embedded fields

> Struct literal construction must include required fields from embeds.

```ds
struct Transform {
    x: int32
}

struct Entity {
    ...Transform
}

const entity = Entity {};
```

- contains: not assignable

### struct embedding rejects duplicate field declarations

> Embedding rejects conflicts when embedded and local fields share names.

```ds
struct Transform {
    x: int32
}

struct Entity {
    ...Transform
    x: string
}
```

- duplicate
