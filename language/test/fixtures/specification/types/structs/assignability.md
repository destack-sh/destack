# Struct Assignability

## nominal types

### struct is not assignable to class with same fields

> Structs remain nominal even when their fields match a class.

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

### class is not assignable to struct with same fields

> Classes remain nominal even when their fields match a struct.

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

### struct satisfies interface with same fields

> Structs can satisfy structural interfaces.

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

### struct satisfies interface with optional fields

> Structs satisfy optional fields structurally.

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

### struct is assignable to object

> Struct values satisfy the top-level object type.

```ds
struct Point {
    x: int32
}

const point = Point { x: 1 };
const value: object = point;
value satisfies object;
```

## interfaces

### struct implements interface

> Structs can implement interfaces and satisfy them structurally.

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

### struct rejects interface when members are missing

> Structs must structurally satisfy interface members.

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

### struct rejects interface when field types mismatch

> Structs must satisfy interface field types.

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

### struct rejects extends

> Structs cannot extend other types.

```ds
struct Base {
    value: int32
}

struct Child extends Base {
    value: int32
}
```

- contains: invalid lineage
