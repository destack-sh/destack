# Class Interfaces

Classes satisfy structural interfaces by shape.
The `implements` clause is an explicit checked declaration.

## implementation

### implementations assign to interfaces

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
