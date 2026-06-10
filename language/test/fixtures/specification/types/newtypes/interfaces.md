# Newtype Interfaces

Newtype interfaces are nominal interfaces.

## implements

### newtypes over interfaces require explicit implements

Newtypes over interfaces create nominal constraints.

```ds
interface Writer {
    write(bytes: readonly uint8[]): uint;
}

newtype NamedWriter = Writer;

struct Buffer {
    write(bytes: readonly uint8[]): uint {
        bytes.length
    }
}

const writer: NamedWriter = Buffer {};
```

- contains: not assignable

### newtypes over interfaces accept explicit implements

Newtypes over interfaces can be implemented explicitly.

```ds
interface Writer {
    write(bytes: readonly uint8[]): uint;
}

newtype NamedWriter = Writer;

struct Buffer {}

extension of Buffer implements NamedWriter {
    write(bytes: readonly uint8[]): uint {
        bytes.length
    }
}

const writer: NamedWriter = Buffer {};
writer.write([]) satisfies uint;
```

### structural matches do not satisfy nominal interfaces

Matching members is not implementing.

```ds
newtype interface Add<T> {
    type Output;

    add(other: T): this.Output;
}

struct Vec2 {
    x: int32;
    y: int32;

    add(other: Vec2): Vec2 {
        Vec2 { x: this.x + other.x, y: this.y + other.y }
    }
}

const value: Add<Vec2> = Vec2 { x: 0, y: 0 };
```

- contains: not assignable

### explicit implements satisfies nominal interfaces

The clause is what implements.

```ds
newtype interface Add<T> {
    type Output;

    add(other: T): this.Output;
}

struct Vec2 {
    x: int32;
    y: int32;
}

extension of Vec2 implements Add<Vec2> {
    type Output = Vec2;

    add(other: Vec2): this.Output {
        Vec2 { x: this.x + other.x, y: this.y + other.y }
    }
}

const value: Add<Vec2> = Vec2 { x: 0, y: 0 };
value.add(Vec2 { x: 1, y: 1 }) satisfies Vec2;
```

### aliases preserve nominal interface identity

An alias names the same interface.

```ds
newtype interface Add<T> {
    type Output;

    add(other: T): this.Output;
}

struct Vec2 {
    x: int32;
    y: int32;
}

type AddVec2 = Add<Vec2>;

extension of Vec2 implements Add<Vec2> {
    type Output = Vec2;

    add(other: Vec2): this.Output {
        Vec2 { x: this.x + other.x, y: this.y + other.y }
    }
}

const value: AddVec2 = Vec2 { x: 0, y: 0 };
value.add(Vec2 { x: 1, y: 1 }) satisfies Vec2;
```

### imported interfaces stay nominal

Nominality crosses modules.

```ds:contract.ds
export newtype interface Add<T> {
    type Output;

    add(other: T): this.Output;
}
```

```ds:main.ds
import { Add } from "./contract.ds";

struct Vec2 {
    x: int32;
    y: int32;

    add(other: Vec2): Vec2 {
        Vec2 { x: this.x + other.x, y: this.y + other.y }
    }
}

const value: Add<Vec2> = Vec2 { x: 0, y: 0 };
```

- contains: not assignable

### imported interfaces accept imported implements

The clause works wherever the interface is nameable.

```ds:contract.ds
export newtype interface Add<T> {
    type Output;

    add(other: T): this.Output;
}
```

```ds:main.ds
import { Add } from "./contract.ds";

struct Vec2 {
    x: int32;
    y: int32;
}

extension of Vec2 implements Add<Vec2> {
    type Output = Vec2;

    add(other: Vec2): this.Output {
        Vec2 { x: this.x + other.x, y: this.y + other.y }
    }
}

const value: Add<Vec2> = Vec2 { x: 0, y: 0 };
value.add(Vec2 { x: 1, y: 1 }) satisfies Vec2;
```
