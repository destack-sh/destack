# Struct Assignability

Structs are nominal value types.

## nominality

### structs do not satisfy classes by shape

```ds
struct Point {
    x: int32
}

class PointClass {
    x: int32 = 0
}

const point = Point { x: 1 };
const value: PointClass = point;
```

- contains: is not assignable

### classes do not satisfy structs by shape

```ds
struct Point {
    x: int32
}

class PointClass {
    x: int32 = 0
}

const point = new PointClass();
const value: Point = point;
```

- contains: is not assignable

## interfaces

### structs satisfy structural interfaces

```ds
interface HasX {
    x: int32
}

struct Point {
    x: int32
}

const point = Point { x: 1 };
const value: HasX = point;
value satisfies HasX;
```

### structs satisfy optional interface fields

```ds
interface HasName {
    name?: string
}

struct Person {
    name: string
}

const person: HasName = Person { name: "Ada" };
person satisfies HasName;
```

### structs satisfy object

```ds
struct Point {
    x: int32
}

const point = Point { x: 1 };
const value: object = point;
value satisfies object;
```

### implements checks interface shape

```ds
interface Drawable {
    draw(): void
}

struct Point implements Drawable {
    x: int32

    draw(): void {}
}

const point: Drawable = Point { x: 1 };
point satisfies Drawable;
```

### interfaces require declared members

```ds
interface Drawable {
    draw(): void
}

struct Point {
    x: int32
}

const point: Drawable = Point { x: 1 };
```

- contains: is not assignable

### interfaces require matching field types

```ds
interface HasX {
    x: int32
}

struct Point {
    x: float32
}

const point: HasX = Point { x: 1.5 };
```

- contains: is not assignable

## inheritance

### structs reject extends

```ds
struct Base {
    value: int32
}

struct Child extends Base {
    value: int32
}
```

- contains: invalid lineage
