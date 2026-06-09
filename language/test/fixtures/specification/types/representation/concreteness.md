# Concreteness

`Concrete` is the capability trait for types that have a concrete representation.

## constraints

### concrete bounds enable layout queries

An explicit `Concrete` bound lets generic code ask layout questions.

```ds
function storageSize<T: Concrete>(): usize {
    const size = comptime sizeOf<T>();
    return size;
}

const size = storageSize<int32>();
size satisfies usize;
```

## aliases

### layout queries do not reify aliases

Layout queries require a representation that has already been selected.

```ds
struct Circle {
    radius: float64;
}

struct Rectangle {
    width: float64;
    height: float64;
}

type Shape = Circle | Rectangle;

const size = comptime sizeOf<Shape>();
```

- contains: no concrete representation

### newtype unions choose storage

A newtype union creates a nominal representation.

```ds
struct Circle {
    radius: float64;
}

struct Rectangle {
    width: float64;
    height: float64;
}

newtype Shape = Circle | Rectangle;

const size = comptime sizeOf<Shape>();
size satisfies usize;
```

## dynamic

### dynamic wrappers are concrete

`Dynamic<T>` is a concrete erased value for any dynamic-safe type surface.

```ds
interface Writer {
    write(bytes: readonly uint8[]): uint;
}

const size = comptime sizeOf<Dynamic<Writer>>();
size satisfies usize;
```

### dynamic wrappers accept anonymous constraints

Anonymous constraints can be erased when they are dynamic-safe.

```ds
type Writer = {
    write(bytes: readonly uint8[]): uint;
};

const size = comptime sizeOf<Dynamic<Writer>>();
size satisfies usize;
```

## returns

### transparent returns hide their concrete representation

A transparent return annotation hides the representation chosen by the function body.

```ds
struct Circle {
    radius: float64;
}

struct Rectangle {
    width: float64;
    height: float64;
}

type Shape = Circle | Rectangle;

function makeCircle(): Shape {
    return Circle { radius: 1.0 };
}

makeCircle() satisfies Shape;
```

### transparent returns reject multiple representations

One monomorphized function cannot return different representations through a transparent annotation.

```ds
struct Circle {
    radius: float64;
}

struct Rectangle {
    width: float64;
    height: float64;
}

type Shape = Circle | Rectangle;

function makeShape(flag: boolean): Shape {
    if (flag) {
        return Circle { radius: 1.0 };
    }

    return Rectangle { width: 1.0, height: 1.0 };
}
```

- contains: one concrete representation

### newtype returns allow multiple variants

A newtype union names the runtime representation.

```ds
struct Circle {
    radius: float64;
}

struct Rectangle {
    width: float64;
    height: float64;
}

newtype Shape = Circle | Rectangle;

function makeShape(flag: boolean): Shape {
    if (flag) {
        return Shape(Circle { radius: 1.0 });
    }

    return Shape(Rectangle { width: 1.0, height: 1.0 });
}

makeShape(true) satisfies Shape;
```
