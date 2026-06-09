# Storage

Storage sites choose representation.

## fields

### type aliases reify field storage

Transparent aliases expand before a field chooses storage.

```ds
type Point = {
    x: int32;
    y: int32;
};

struct Rectangle {
    start: Point;
}

const rectangle = Rectangle {
    start: { x: 0, y: 0 },
};

rectangle.start satisfies Point;
rectangle.start.x satisfies int32;
```

### alias fields reject extra storage fields

Exact field storage rejects fields outside the aliased shape.

```ds
type Point = {
    x: int32;
    y: int32;
};

struct Rectangle {
    start: Point;
}

const rectangle = Rectangle {
    start: { x: 0, y: 0, z: 0 },
};
```

- contains: excess property

### interface fields induce generics

Named constraints keep the concrete field type generic.

```ds
interface PointLike {
    x: int32;
    y: int32;
}

struct Rectangle {
    start: PointLike;
    end: PointLike;
}

struct Point implements PointLike {
    x: int32;
    y: int32;
}

struct Offset implements PointLike {
    x: int32;
    y: int32;
}

const rectangle = Rectangle {
    start: Point { x: 0, y: 0 },
    end: Offset { x: 1, y: 1 },
};

rectangle.start satisfies Point;
rectangle.end satisfies Offset;
```

## containers

### array elements reify aliases

Array elements choose storage for their type argument.

```ds
struct Circle {
    radius: float64;
}

struct Rectangle {
    width: float64;
    height: float64;
}

type Shape = Circle | Rectangle;

const size = comptime sizeOf<Array<Shape>>();
size satisfies usize;
```
