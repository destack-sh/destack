# Class Interfaces

Classes satisfy structural interfaces by shape.
The `implements` clause is an explicit checked declaration.

## implementation

### implementations assign to interfaces

An implementor flows into the interface type.

```ds
interface Printable {
    print(): void;
}

class Document implements Printable {
    print(): void {}
}

declare function getDocument(): Document;

const printable: Printable = getDocument();
```

### implementations satisfy interfaces

`satisfies` follows the implementation.

```ds
interface Printable {
    print(): void;
}

class Document implements Printable {
    print(): void {}
}

declare function getDocument(): Document;

getDocument() satisfies Printable;
```

### interfaces do not assign to implementations

The interface is wider than any implementor.

```ds
interface Printable {
    print(): void;
}

class Document implements Printable {
    print(): void {}
}

declare function getPrintable(): Printable;

const document: Document = getPrintable();
```

- contains: not assignable

## function parameters

### implementations pass to interface parameters

Parameter positions accept implementors.

```ds
interface Printable {
    print(): void;
}

class Document implements Printable {
    print(): void {}
}

function acceptPrintable(p: Printable): void {}

declare function getDocument(): Document;

acceptPrintable(getDocument());
```

## multiple interfaces

### classes implement multiple interfaces

Each clause is checked independently.

```ds
interface Printable {
    print(): void;
}

interface Saveable {
    save(): void;
}

class Document implements Printable, Saveable {
    print(): void {}
    save(): void {}
}

declare function getDocument(): Document;

const printable: Printable = getDocument();
const saveable: Saveable = getDocument();
```

## mixed inheritance

### classes extend and implement together

Extension and implementation compose.

```ds
class Base {
    id: number = 0;
}

interface Printable {
    print(): void;
}

class Document extends Base implements Printable {
    print(): void {}
}

declare function getDocument(): Document;

const base: Base = getDocument();
const printable: Printable = getDocument();
```

## structural subtyping

### classes satisfy interfaces structurally

Plain interfaces match by structure, no clause needed.

```ds
interface HasId {
    id: number;
}

class Document {
    id: number = 0;
}

declare function getDocument(): Document;

const hasId: HasId = getDocument();
```

### interfaces require class members

Missing members fail the match.

```ds
interface HasId {
    id: number;
}

class Document {
    name: string = "";
}

declare function getDocument(): Document;

const hasId: HasId = getDocument();
```

- contains: not assignable

### structs satisfy interfaces structurally

Structs match plain interfaces the same way.

```ds
interface HasName {
    name: string;
}

struct Person {
    name: string;
    age: number;
}

declare function getPerson(): Person;

const named: HasName = getPerson();
```

### extra class fields are allowed

Structural matching ignores extra members.

```ds
interface Named {
    name: string;
}

class User {
    name: string = "";
    email: string = "";
    age: number = 0;
}

declare function getUser(): User;

const named: Named = getUser();
```
