# Stability

Writing through a non-exclusive borrow is only allowed when the place is overwrite-stable: the old value needs no destruction, and the new bytes mean what the old bytes meant.

## stable

### scalar fields stay writable through borrows

Scalar overwrites cannot invalidate anything.

```ds
struct Point {
    x: int32;
}

let point = ^Point { x: 1 };
let borrow = &point;

borrow.x = 2;
```

### managed reference fields stay writable through borrows

Replacing a managed handle keeps the old referent alive for the garbage collector.

```ds
class User {}

class Session {
    user: User = new User();
}

let session: Session = new Session();
let borrow = &session;

borrow.user = new User();
```

## unstable

### overwriting owned interiors requires exclusivity

Overwriting a value with drop glue could free memory a live borrow still targets.

```ds
struct Data {
    value: int32;
}

struct Frame {
    data: ^Data;
}

let frame = ^Frame { data: ^Data { value: 1 } };
let borrow = &frame;

borrow.data = ^Data { value: 2 };
```

- contains: exclusive

### overwriting variants requires exclusivity

Overwriting a variant in place changes what its payload bytes mean.

```ds
@derive(Tagged)
newtype Shape =
    | { kind: "circle"; radius: float64 }
    | { kind: "square"; side: float64 };

struct Scene {
    shape: Shape;
}

let scene = ^Scene { shape: Shape.Circle({ radius: 1.0 }) };
let borrow = &scene;

borrow.shape = Shape.Square({ side: 2.0 });
```

- contains: exclusive

### exclusive borrows may overwrite anything

Exclusivity guarantees no other borrow can observe the replacement.

```ds
struct Data {
    value: int32;
}

struct Frame {
    data: ^Data;
}

let frame = ^Frame { data: ^Data { value: 1 } };
let write = &exclusive frame;

write.data = ^Data { value: 2 };
```

## managed

### managed variant fields stay writable

Managed objects keep variant payloads behind GC indirection, so replacement is just a handle swap.

```ds
@derive(Tagged)
newtype Shape =
    | { kind: "circle"; radius: float64 }
    | { kind: "square"; side: float64 };

class Scene {
    shape: Shape = Shape.Circle({ radius: 1.0 });
}

let scene: Scene = new Scene();
let alias = scene;

scene.shape = Shape.Square({ side: 2.0 });
alias.shape satisfies Shape;
```
