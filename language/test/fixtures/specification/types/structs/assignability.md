# Struct Assignability

Structs are nominal value types.

## nominality

### structs do not satisfy classes by shape

Structs and classes are different representations.

```ds
struct Point {
    x: int32;
}

class PointClass {
    x: int32 = 0;
}

const point = Point { x: 1 };
const value: PointClass = point;
```

- contains: is not assignable

### classes do not satisfy structs by shape

The separation cuts both ways.

```ds
struct Point {
    x: int32;
}

class PointClass {
    x: int32 = 0;
}

const point = new PointClass();
const value: Point = point;
```

- contains: is not assignable

## interfaces

### structs satisfy structural interfaces

Structs match plain interfaces by shape.

```ds
interface HasX {
    x: int32;
}

struct Point {
    x: int32;
}

const point = Point { x: 1 };
const value: HasX = point;
value satisfies HasX;
```

### structs satisfy optional interface fields

Missing optional members still match.

```ds
interface HasName {
    name?: string;
}

struct Person {
    name: string;
}

const person: HasName = Person { name: "Ada" };
person satisfies HasName;
```

### structs satisfy structural fields

Struct values flow into structural object types.

```ds
struct Point {
    x: int32;
}

const point = Point { x: 1 };
const value: { readonly x: int32 } = point;
value satisfies { readonly x: int32 };
```

### implements checks interface shape

An `implements` clause is verified, not asserted.

```ds
interface Drawable {
    draw(): void;
}

struct Point implements Drawable {
    x: int32;

    draw(): void {}
}

const point: Drawable = Point { x: 1 };
point satisfies Drawable;
```

### interfaces require declared members

Missing members fail the clause.

```ds
interface Drawable {
    draw(): void;
}

struct Point {
    x: int32;
}

const point: Drawable = Point { x: 1 };
```

- contains: is not assignable

### interfaces require matching field types

Field types must line up exactly.

```ds
interface HasX {
    x: int32;
}

struct Point {
    x: float32;
}

const point: HasX = Point { x: 1.5 };
```

- contains: is not assignable
