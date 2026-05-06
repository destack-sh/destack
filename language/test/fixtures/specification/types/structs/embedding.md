# Struct Embedding

Embedded structs contribute fields directly to the outer struct.

## fields

### embedding exposes fields

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

### multiple embeds expose fields

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

### nested embeds expose fields

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

### deep embeds expose fields

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

### embedded fields are required

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

### embedding rejects duplicate fields

```ds
struct Transform {
    x: int32
}

struct Entity {
    ...Transform
    x: string
}
```

- contains: duplicate

### embedding rejects classes

Only structs can be embedded into structs.

```ds
class Transform {
    x: int32 = 0
}

struct Entity {
    ...Transform
}
```

- contains: embed
