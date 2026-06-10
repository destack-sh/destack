# Owned

`^T` is single-owner storage with deterministic cleanup.

## construction

### owned expression yields owned value

Owned expressions create `^T`.

```ds
struct Point {
    x: int32;
}

let point = ^Point { x: 1 };

point satisfies ^Point;
```

### plain values do not satisfy owned destinations

Owned destinations require ownership.

```ds
struct Point {
    x: int32;
}

let point: ^Point = Point { x: 1 };
```

- contains: not assignable

### owned parameters consume owned values

Passing a value to an owned parameter moves it.

```ds
struct Point {
    x: int32;
}

function consume(point: ^Point): void {
    point.x;
}

let point = ^Point { x: 1 };
consume(point);
point.x;
```

- contains: use of moved value

### owned conversion moves plain values

`^expr` converts a plain value into an owned value.

```ds
struct Point {
    x: int32;
}

function consume(point: ^Point): void {
    point.x;
}

let point = Point { x: 1 };
consume(^point);
```

### owned conversion rejects owned values

An owned value cannot be owned again.

```ds
struct Point {
    x: int32;
}

let point = ^Point { x: 1 };
let again = ^point;
```

- contains: ownership operator requires an unowned value

## fields

### structs can store owned fields

Owned fields keep their ownership form.

```ds
struct Data {
    value: int32;
}

struct Container {
    data: ^Data;
}

const container = Container { data: ^Data { value: 1 } };

container.data satisfies ^Data;
```

## readonly

### readonly owned values forbid mutation

`^readonly T` owns storage but forbids mutation through that handle.

```ds
struct Point {
    x: int32;
}

let point: ^readonly Point = ^readonly Point { x: 1 };
point.x = 2;
```

- contains: cannot assign

### readonly owned values are deep

Readonly owned handles protect nested fields.

```ds
struct Profile {
    name: string;
}

struct User {
    profile: Profile;
}

let user: ^readonly User = ^readonly User {
    profile: Profile { name: "Ada" },
};

user.profile.name = "Grace";
```

- contains: cannot assign
