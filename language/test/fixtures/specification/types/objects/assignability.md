# Object Assignability

## structural assignability

### object literal satisfies interface

Object literals are assignable to compatible interfaces.

```ds
interface Person {
    name: string;
}

const person = { name: "Ada" };
person satisfies Person;
```

### missing required field is not assignable

Object literals missing required fields are not assignable.

```ds
interface Person {
    name: string;
    age: number;
}

const person = { name: "Ada" };
person satisfies Person;
```

- contains: not assignable

### excess property reports error

Extra fields are rejected for object literals assigned to interfaces.

```ds
interface Person {
    name: string;
}

const person: Person = { name: "Ada", age: 42 };
```

- contains: excess property

### extra fields allowed for non-literals

Non-literal values are structurally assignable even with extra fields.

```ds
interface Person {
    name: string;
}

const raw = { name: "Ada", age: 42 };
raw satisfies Person;
```

### optional fields allow omission

Optional fields can be omitted in object literals.

```ds
interface Person {
    name?: string;
}

const person = {};
person satisfies Person;
```

### optional fields reject explicit undefined

```ds
interface Target {
    value?: number;
}

const value: Target = { value: undefined };
```

- contains: not assignable

### optional fields are not assignable to required

Optional fields are not assignable to required fields.

```ds
interface Target {
    name: string;
}

interface Source {
    name?: string;
}

const source: Source = {};
source satisfies Target;
```

- contains: not assignable

### required fields are assignable to optional

Required fields are assignable to optional fields.

```ds
interface Target {
    name?: string;
}

interface Source {
    name: string;
}

const source: Source = { name: "Ada" };
source satisfies Target;
```

### assignment rejects optional to required

Assignments reject optional fields when required is expected.

```ds
interface Target {
    name: string;
}

interface Source {
    name?: string;
}

const source: Source = {};
const target: Target = source;
```

- contains: not assignable

## assignment allows required to optional

### assignment allows required to optional

Assignments allow required fields when optional is expected.

```ds
interface Target {
    name?: string;
}

interface Source {
    name: string;
}

const source: Source = { name: "Ada" };
const target: Target = source;
```

### interface declarations merge in declaration files

Declarations in .ds merge into a single interface.

```ds:types.ds
export interface Widget {
    value: number;
}

export interface Widget {
    label: string;
}

export const widget: Widget;
```

```ds:main.ds
import { widget } from "./types.ds";

widget.label satisfies string;
widget.value satisfies number;
```

### duplicate interface names are rejected in .ds

Duplicate interface declarations are rejected outside declaration files.

```ds
interface Duplicate {
    value: number;
}

interface Duplicate {
    label: string;
}
```

- contains: duplicate identifier

### interface assignability is structural

Compatible interfaces are assignable based on shape.

```ds
interface Named {
    name: string;
}

interface Person {
    name: string;
}

const person: Person = { name: "Ada" };
person satisfies Named;
```

## nominal interfaces

### nominal interface requires explicit implements

Nominal interfaces are not satisfied structurally.

```ds
newtype interface Add<T> {
    type Output;

    add(other: T): this.Output;
}

struct Vec2 {
    x: float32;
    y: float32;

    add(other: Vec2): Vec2 {
        Vec2 { x: this.x + other.x, y: this.y + other.y }
    }
}

const value: Add<Vec2> = Vec2 { x: 1, y: 2 };
```

- contains: not assignable

### nominal interface accepts explicit implements

Nominal interfaces require an explicit implements clause.

```ds
newtype interface Add<T> {
    type Output;

    add(other: T): this.Output;
}

struct Vec2 {
    x: float32;
    y: float32;
}

extension of Vec2 implements Add<Vec2> {
    type Output = Vec2;

    add(other: Vec2): this.Output {
        Vec2 { x: this.x + other.x, y: this.y + other.y }
    }
}

const value: Add<Vec2> = Vec2 { x: 1, y: 2 };
value.add(Vec2 { x: 2, y: 3 }) satisfies Vec2;
```
