# Mapped Type Modifiers

Mapped type modifiers and key remapping should follow TypeScript behavior.
Key remapping should operate on apparent keys and modifiers should be applied after key selection.

## Modifiers

### mapped modifiers can remove readonly and optional markers

> Removing readonly and optional markers should require all fields to be present.

```ds
interface Person {
    readonly name: string
    age?: number
}

type MutableRequired<T> = { -readonly [K in keyof T]-?: T[K] };

const bad: MutableRequired<Person> = {};
```

- contains: not assignable

### mapped readonly modifiers prevent mutation

> Adding readonly markers should prevent writes through the mapped type.

```ds
type Frozen<T> = { readonly [K in keyof T]: T[K] };

const value: Frozen<{ a: number }> = { a: 1 };
value.a = 2;
```

- contains: readonly

## Key remapping

### key remapping to never removes keys

> Remapping keys to `never` should drop those keys from the resulting type.

```ds
interface Shape {
    a: number
    b: string
}

type WithoutA<T> = { [K in keyof T as K extends "a" ? never : K]: T[K] };

const ok: WithoutA<Shape> = { b: "x" };
const bad: WithoutA<Shape> = { a: 1, b: "x" };
```

- contains: not assignable

### key remap collisions merge value types

> Remapping multiple keys to the same key should merge their value types.

```ds
interface Shape {
    a: number
    b: string
}

type Merge<T> = { [K in keyof T as "value"]: T[K] };

const bad: Merge<Shape> = { value: true };
```

- contains: not assignable
