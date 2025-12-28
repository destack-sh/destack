# Interfaces

## object literal satisfies interface

> Object literals are assignable to compatible interfaces.

```ds
interface Person {
    name: string
}

const person = { name: "Ada" }
person satisfies Person;
```

## missing required field is not assignable

> Object literals missing required fields are not assignable.

```ds
interface Person {
    name: string
    age: number
}

const person = { name: "Ada" }
person satisfies Person;
```

- contains: not assignable

## excess property reports error

> Extra fields are rejected for object literals assigned to interfaces.

```ds
interface Person {
    name: string
}

const person: Person = { name: "Ada", age: 42 };
```

- contains: excess property

## extra fields allowed for non-literals

> Non-literal values are structurally assignable even with extra fields.

```ds
interface Person {
    name: string
}

const raw = { name: "Ada", age: 42 }
raw satisfies Person;
```

## optional fields allow omission

> Optional fields can be omitted in object literals.

```ds
interface Person {
    name?: string
}

const person = {}
person satisfies Person;
```

## exactOptionalPropertyTypes forbids undefined assignment

```ds
interface Target {
    value?: number
}

const value: Target = { value: undefined }
```

- contains: not assignable

## exactOptionalPropertyTypes false allows undefined assignment

```ds:dsconfig.json
{ "compilerOptions": { "exactOptionalPropertyTypes": false } }
```

```ds:package.json
{ "name": "spec" }
```

```ds
interface Target {
    value?: number
}

const value: Target = { value: undefined }
```

## optional fields are not assignable to required

> Optional fields are not assignable to required fields.

```ds
interface Target {
    name: string
}

interface Source {
    name?: string
}

const source: Source = {}
source satisfies Target;
```

- contains: not assignable

## required fields are assignable to optional

> Required fields are assignable to optional fields.

```ds
interface Target {
    name?: string
}

interface Source {
    name: string
}

const source: Source = { name: "Ada" }
source satisfies Target;
```

## assignment rejects optional to required

> Assignments reject optional fields when required is expected.

```ds
interface Target {
    name: string
}

interface Source {
    name?: string
}

const source: Source = {}
const target: Target = source
```

- contains: not assignable

## assignment allows required to optional

> Assignments allow required fields when optional is expected.

```ds
interface Target {
    name?: string
}

interface Source {
    name: string
}

const source: Source = { name: "Ada" }
const target: Target = source
```

## interface assignability is structural

> Compatible interfaces are assignable based on shape.

```ds
interface Named {
    name: string
}

interface Person {
    name: string
}

const person: Person = { name: "Ada" }
person satisfies Named;
```
