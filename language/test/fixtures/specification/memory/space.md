# Space

Placement decides which heap a value lives in: Worker-local by default, `shared` across Workers.

## borrows

### shared borrow reads shared storage

`shared &T` is borrowed access to shared storage.

```ds
struct Point {
    x: int32;
    y: int32;
}

function kernel(data: shared &Point): int32 {
    data.x
}
```

## values

### local places values in local space

`local T` explicitly rebases `T` into Worker-local space.

```ds
struct Point {
    x: int32;
    y: int32;
}

let point: local Point = Point { x: 1, y: 2 };
point satisfies WithSpace<Point, "local">;
```

### the Local wrapper places values in local space

`Local<T>` is the wrapper spelling of the same rebase.

```ds
struct Point {
    x: int32;
    y: int32;
}

let point: Local<Point> = Point { x: 1, y: 2 };
point satisfies local Point;
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

### placed declarations give nominal types an intrinsic place

`local` and `shared` on nominal declarations define the place of that nominal type.

```ds
local class Promise<T> {}
local struct Continuation<T> {}
shared class Channel<T> {}
shared enum Delivery {
    Pending;
    Complete;
}
local newtype interface Awaitable<T> {}
local newtype TaskId = uint64;

PlaceOf<Promise<void>> satisfies "local";
PlaceOf<Continuation<void>> satisfies "local";
PlaceOf<Channel<string>> satisfies "shared";
PlaceOf<Delivery> satisfies "shared";
PlaceOf<TaskId> satisfies "local";
```

### placed newtype interfaces constrain implementors

`local` and `shared` on a newtype interface constrain explicit implementors to that place.

```ds
local newtype interface Awaitable<T> {
    await(): T;
}

local class Promise<T> implements Awaitable<T> {
    await(): T {
        todo("...")
    }
}
```

### structural interfaces reject explicit placement

Structural interfaces describe constraints without explicit implementor edges.

```ds
local interface Waitable {}
```

- contains: interface

### local values can hold shared values

Local storage can hold handles to shared values.

```ds
class Registry {}

const registry: shared Registry = new Registry();
const holder = { registry };

holder.registry satisfies shared Registry;
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
