# Interface Implementation (implements)

Interface implementation using `implements`.

## implementation

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
    id: number = 0
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

## structural subtyping

### class without implements assignable to interface

> A class with the same shape as an interface is assignable without explicit `implements`.

```ds
interface HasId {
    id: number
}

class Document {
    id: number = 0
}

declare function getDocument(): Document;

const hasId: HasId = getDocument();
```

### class missing field not assignable to interface

> A class that lacks a required field is not assignable to the interface.

```ds
interface HasId {
    id: number
}

class Document {
    name: string = ""
}

declare function getDocument(): Document;

const hasId: HasId = getDocument();
```

- type Document is not assignable to type HasId

### struct without implements assignable to interface

> Structs also support structural subtyping to interfaces.

```ds
interface HasName {
    name: string
}

struct Person {
    name: string
    age: number
}

declare function getPerson(): Person;

const named: HasName = getPerson();
```

### class with extra fields assignable to interface

> A class with more fields than required is still assignable.

```ds
interface Named {
    name: string
}

class User {
    name: string = ""
    email: string = ""
    age: number = 0
}

declare function getUser(): User;

const named: Named = getUser();
```

## invalid implements

### classes cannot declare empty implements clauses

> Classes cannot declare empty implements clauses.

```ds
class Counter implements {
}
```

- invalid lineage
