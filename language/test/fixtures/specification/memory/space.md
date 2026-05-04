# Space

Space fixtures cover local and shared placement.

## annotations

### space annotation applies to borrowed references

> Explicit space annotations use `@space`.

```ds
struct Point {
    x: int32;
    y: int32;
}

function kernel(data: @space("shared") &Point): int32 {
    data.x
}
```

### shared types place values in shared space

> `shared T` rebases `T` into shared space.

```ds
struct Point {
    x: int32;
    y: int32;
}

let point: shared Point = Point { x: 1, y: 2 };
point satisfies shared Point;
```

### shared aggregate fields inherit shared placement

> Ambient fields follow the placement of their containing aggregate.

```ds
struct Header {
    id: int32;
}

struct Payload {
    value: int32;
}

struct Request<T> {
    header: Header;
    body: T;
}

let request: shared Request<Payload>;
request.header satisfies shared Header;
request.body satisfies shared Payload;
```

### shared values cannot contain explicit local fields

> Shared values cannot point directly into a Worker-local heap.

```ds
struct Payload {
    value: int32;
}

struct Request {
    body: WithSpace<Payload, "local">;
}

let request: shared Request;
```

- contains: shared
