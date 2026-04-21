# Extensions Implementing Interfaces

Tests for extensions that implement interfaces.

## Basic Interface Implementation

### extension implements interface

> Extensions can implement interfaces for types.

```ds
interface Describable {
    describe(): string
}

struct Point { x: number; y: number }

extension of Point implements Describable {
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

extension of Document implements Printable, Serializable {
    print(): void {}
    serialize(): string { return "" }
}

declare function getDocument(): Document;

const document = getDocument();
document.print();
document.serialize() satisfies string;
```

### extension rejects missing interface members

> Extensions that implement interfaces must provide all required members.

```ds
interface Printable {
    print(): void
}

interface Serializable {
    serialize(): string
}

struct Document { content: string }

extension of Document implements Printable, Serializable {
    print(): void {}
}
```

- missing implementation

### extension can implement imported interfaces across modules

> Extensions can satisfy imported interface contracts across module boundaries.

```ds:contracts.ds
export interface Printable {
    print(): string
}
```

```ds:model.ds
export struct Document { content: string }
```

```ds:main.ds
import { Printable } from "./contracts";
import { Document } from "./model";

extension of Document implements Printable {
    print(): string { return this.content }
}

declare function getDocument(): Document;

const document = getDocument();
document.print() satisfies string;
```

### extension implementing imported contracts rejects missing members

> Imported interface contracts still require all members in extension implementations.

```ds:contracts.ds
export interface Printable {
    print(): string
}

export interface Serializable {
    serialize(): string
}
```

```ds:model.ds
export struct Document { content: string }
```

```ds:main.ds
import { Printable, Serializable } from "./contracts";
import { Document } from "./model";

extension of Document implements Printable, Serializable {
    print(): string { return this.content }
}
```

- missing implementation
