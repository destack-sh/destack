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

