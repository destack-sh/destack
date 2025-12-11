# Interface Implementation (implements)

Tests for interface implementation using `implements`.

## Basic Implementation

### implementor assignable to interface

> A class that implements an interface is assignable to the interface type.

```ds
interface Printable {
    print(): void
}

class Document implements Printable {
    print(): void {}
}

declare function getDocument(): Document;

const printable: Printable = getDocument();
```

### implementor satisfies interface

> A class that implements an interface satisfies the interface type.

```ds
interface Printable {
    print(): void
}

class Document implements Printable {
    print(): void {}
}

declare function getDocument(): Document;

getDocument() satisfies Printable;
```

### interface not assignable to implementor

> An interface type is not assignable to a concrete implementor type.

```ds
interface Printable {
    print(): void
}

class Document implements Printable {
    print(): void {}
}

declare function getPrintable(): Printable;

const document: Document = getPrintable();
```

- type Printable is not assignable to type Document

## Function Parameters

### implementor passed to interface parameter

> A class can be passed where its implemented interface is expected.

```ds
interface Printable {
    print(): void
}

class Document implements Printable {
    print(): void {}
}

function acceptPrintable(p: Printable): void {}

declare function getDocument(): Document;

acceptPrintable(getDocument());
```

## Multiple Interfaces

### multiple interfaces implemented

> A class can implement multiple interfaces.

```ds
interface Printable {
    print(): void
}

interface Saveable {
    save(): void
}

class Document implements Printable, Saveable {
    print(): void {}
    save(): void {}
}

declare function getDocument(): Document;

const printable: Printable = getDocument();
const saveable: Saveable = getDocument();
```

## Mixed Inheritance

### class extends and implements

> A class can extend a class and implement interfaces.

```ds
class Base {
    id: number
}

interface Printable {
    print(): void
}

class Document extends Base implements Printable {
    print(): void {}
}

declare function getDocument(): Document;

const base: Base = getDocument();
const printable: Printable = getDocument();
```
