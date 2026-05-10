# Space

## borrows

### shared borrow reads shared storage

`shared &T` is borrowed access to shared storage.

```ds
struct Point {
    x: int32;
    y: int32;
}

function kernel(data: shared (&Point)): int32 {
    data.x
}
```

## values

### local is an algebra form

`Local<T>` explicitly pins a type to Worker-local space.

```ds
struct Point {
    x: int32;
    y: int32;
}

let point: Local<Point> = Point { x: 1, y: 2 };
point satisfies WithSpace<Point, "local">;
```

### shared places values in shared space

`shared T` rebases `T` into shared space.

```ds
struct Point {
    x: int32;
    y: int32;
}

let point: shared Point = Point { x: 1, y: 2 };
point satisfies shared Point;
```

### ambient fields inherit shared placement

Ambient fields follow the placement of their containing aggregate.

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

### shared values reject explicit local fields

Shared values cannot point directly into a Worker-local heap.

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
