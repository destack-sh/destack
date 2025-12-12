# Extensions Implementing Interfaces

Tests for extensions that implement interfaces.

> NOTE: Operator overloading via interface implementation is not yet supported.
> These tests cover the syntax and basic semantics of `extension T implements I { }`.

## Basic Interface Implementation

### extension implements interface

> Extensions can implement interfaces for types.

```ds
interface Describable {
    describe(): string
}

struct Point { x: number, y: number }

extension Point implements Describable {
    describe(): string {
        return ""
    }
}

declare function getPoint(): Point;

const point = getPoint();
point.describe() satisfies string;
```

### extension implements multiple interfaces

> Extensions can implement multiple interfaces.

```ds
interface Printable {
    print(): void
}

interface Serializable {
    serialize(): string
}

struct Document { content: string }

extension Document implements Printable, Serializable {
    print(): void {}
    serialize(): string { return "" }
}

declare function getDocument(): Document;

const document = getDocument();
document.print();
document.serialize() satisfies string;
```

### separate extensions implement different interfaces

> A type can have multiple extensions, each implementing different interfaces.

```ds
interface Printable {
    print(): void
}

interface Cloneable {
    clone(): this
}

struct Document { content: string }

extension Document implements Printable {
    print(): void {}
}

extension Document implements Cloneable {
    clone(): Document { return Document { content: this.content } }
}

declare function getDocument(): Document;

const document = getDocument();
document.print();
document.clone() satisfies Document;
```

## Interface with Methods

### extension implements interface with default method

> Extensions can implement interfaces that have default implementations.

```ds
interface Comparable {
    compare(other: this): number

    isLessThan(other: this): boolean {
        return this.compare(other) < 0
    }
}

struct Integer { value: number }

extension Integer implements Comparable {
    compare(other: Integer): number {
        return this.value - other.value
    }
}

declare function getInteger(): Integer;

const first = getInteger();
const second = getInteger();
first.compare(second) satisfies number;
first.isLessThan(second) satisfies boolean;
```

## Generic Interface Implementation

### extension implements generic interface

> Extensions can implement generic interfaces.

```ds
interface Wrapper<T> {
    wrap(value: T): this
    unwrap(): T
}

struct NumberBox { value: number }

extension NumberBox implements Wrapper<number> {
    wrap(value: number): NumberBox { return NumberBox { value: value } }
    unwrap(): number { return this.value }
}

declare function getNumberBox(): NumberBox;

const box = getNumberBox();
box.unwrap() satisfies number;
```

### extension implements interface with same type parameter

> Extension on generic type implements interface using the same type parameter.

```ds
interface Container<T> {
    get(): T
    set(value: T): void
}

struct Holder<T> { value: T }

extension Holder<T> implements Container<T> {
    get(): T { return this.value }
    set(value: T): void {}
}

declare function getStringHolder(): Holder<string>;

const holder = getStringHolder();
holder.get() satisfies string;
```

## Named Extension Implementing Interface

### named extension implements interface

> Named extensions can implement interfaces.

```ds
interface Hashable {
    hash(): number
}

struct Point { x: number, y: number }

extension PointHash: Point implements Hashable {
    hash(): number { return this.x + this.y }
}

declare function getPoint(): Point;

const point = getPoint();
point.hash() satisfies number;
```

## Extension Adds Interface to Foreign Type

### local extension implements interface on foreign type

> Extensions can add interface implementations to imported types.

```ds:types.ds
export struct Vector2 { x: number, y: number }
```

```ds:main.ds
import { Vector2 } from "./types.ds"

interface Stringable {
    toString(): string
}

extension Vector2 implements Stringable {
    toString(): string { return "" }
}

declare function getVector(): Vector2;

const vector = getVector();
vector.toString() satisfies string;
```
