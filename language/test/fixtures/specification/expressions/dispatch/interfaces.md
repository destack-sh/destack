# Interface Specialization

Interface annotations specialize to the concrete value at each application.

## fields

### field order does not matter

Interface field access uses member names, not concrete layout order.

```ds
interface PointLike {
    x: int32;
    y: int32;
}

struct Point implements PointLike {
    y: int32;
    x: int32;
}

const point: PointLike = Point { y: 2, x: 1 };

const total = point.x + point.y;
total satisfies int32;
```

### structural fields do not require implements

Structural interfaces accept matching fields.

```ds
interface Named {
    name: string;
}

struct User {
    name: string;
}

const user: Named = User { name: "Ada" };
user.name satisfies string;
```

### mutable fields are read-write

Interface fields are mutable by default.

```ds
interface PointLike {
    x: int32;
}

struct Point {
    x: int32;
}

let point: PointLike = Point { x: 1 };
point.x = 2;
point.x satisfies int32;
```

### readonly fields can widen

Readonly interface fields can use ordinary read casts.

```ds
interface PointLike {
    readonly x: int32 | float32;
}

struct Point {
    x: int32;
}

const point: PointLike = Point { x: 1 };
point.x satisfies int32 | float32;
```

### mutable fields reject widened writes

Mutable interface fields must accept writes through the interface type.

```ds
interface PointLike {
    x: int32 | float32;
}

struct Point {
    x: int32;
}

const point: PointLike = Point { x: 1 };
```

- contains: is not assignable

### accessors can adapt mutable views

Accessors can provide explicit read-write adaptation.

```ds
interface PointLike {
    get x(): int32 | float32;
    set x(value: int32 | float32);
}

class Point implements PointLike {
    private value: int32 = 0;

    get x(): int32 | float32 {
        this.value
    }

    set x(value: int32 | float32) {
        this.value = value as int32;
    }
}

const point: PointLike = new Point();
point.x = 2.0;
point.x satisfies int32 | float32;
```

### nested fields can widen

Nested readonly fields use the same interface view rules.

```ds
interface PointLike {
    readonly x: int32 | float32;
}

interface ShapeLike {
    readonly origin: PointLike;
}

struct Point {
    x: int32;
}

struct Shape {
    origin: Point;
}

const shape: ShapeLike = Shape { origin: Point { x: 1 } };
shape.origin.x satisfies int32 | float32;
```

## methods

### methods specialize through interfaces

Interface method calls use the concrete method target.

```ds
interface Writer {
    write(chunk: [byte]): usize;
}

struct Buffer implements Writer {
    write(chunk: [byte]): usize {
        chunk.length
    }
}

declare const bytes: [byte];

const writer: Writer = Buffer {};
const written = writer.write(bytes);
written satisfies usize;
```

### nominal interfaces require implements

Newtype interfaces are not satisfied structurally.

```ds
newtype interface Writer {
    write(chunk: [byte]): usize;
}

struct Buffer {
    write(chunk: [byte]): usize {
        chunk.length
    }
}

const writer: Writer = Buffer {};
```

- contains: is not assignable

## indexes

### index signatures dispatch through interfaces

Interface index access keeps the index signature result type.

```ds
interface Bag<T> {
    [key: string]: T | undefined;
}

declare const bag: Bag<int32>;
declare const key: string;

const value = bag[key];
value satisfies int32 | undefined;
```

### records satisfy string index signatures

Records can be viewed through structural index signatures.

```ds
interface Bag<T> {
    [key: string]: T | undefined;
}

const bag: Bag<int32> = { alpha: 1, beta: 2 };

const value = bag["alpha"];
value satisfies int32 | undefined;
```

### structs satisfy readonly index signatures

Fixed object-shaped values can provide readonly index views.

```ds
interface Bag<T> {
    readonly [key: string]: T | undefined;
}

struct Point {
    x: int32;
    y: int32;
}

const bag: Bag<int32> = Point { x: 1, y: 2 };

const value = bag["x"];
value satisfies int32 | undefined;
```

### classes satisfy readonly index signatures

Class fields can provide readonly index views.

```ds
interface Bag<T> {
    readonly [key: string]: T | undefined;
}

class Point {
    x: int32 = 1;
    y: int32 = 2;
}

const bag: Bag<int32> = new Point();

const value = bag["y"];
value satisfies int32 | undefined;
```

### structs reject mutable index signatures

Fixed object-shaped values do not provide indexed writes.

```ds
interface Bag<T> {
    [key: string]: T | undefined;
}

struct Point {
    x: int32;
    y: int32;
}

const bag: Bag<int32> = Point { x: 1, y: 2 };
```

- contains: is not assignable

### classes reject mutable index signatures

Class fields do not provide indexed writes.

```ds
interface Bag<T> {
    [key: string]: T | undefined;
}

class Point {
    x: int32 = 1;
    y: int32 = 2;
}

const bag: Bag<int32> = new Point();
```

- contains: is not assignable
