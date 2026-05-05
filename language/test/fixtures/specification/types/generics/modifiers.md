# Mapped Type Modifiers

Mapped type modifiers and key remapping operate on apparent keys.
Modifiers are applied after key selection.

## modifiers

### mapped modifiers can remove readonly and optional markers

> Removing readonly and optional markers requires all fields to be present.

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

> Adding readonly markers prevents writes through the mapped type.

```ds
type Frozen<T> = { readonly [K in keyof T]: T[K] };

const value: Frozen<{ a: number }> = { a: 1 };
value.a = 2;
```

- contains: readonly

## key remapping

### key remapping to never removes keys

> Remapping keys to `never` drops those keys from the resulting type.

```ds
interface Shape {
    a: number
    b: string
}

type WithoutA<T> = { [K in keyof T as K extends "a" ? never : K]: T[K] };

const ok: WithoutA<Shape> = { b: "x" };
const bad: WithoutA<Shape> = { a: 1, b: "x" };
```

- contains: excess property

### key remap collisions merge value types

> Remapping multiple keys to the same key merges their value types.

```ds
interface Shape {
    a: number
    b: string
}

type Merge<T> = { [K in keyof T as "value"]: T[K] };

const bad: Merge<Shape> = { value: true };
```

- contains: not assignable

### key remapping supports template literal keys

> Remapped keys can be produced by template literal expressions.

```ds
interface Shape {
    a: number
    b: string
}

type Prefixed<T> = {
    [K in keyof T as K extends string ? `get_${K}` : never]: () => T[K]
};

const ok: Prefixed<Shape> = {
    get_a: () => 1,
    get_b: () => "ok",
};
```

### key remapping template keys reject mismatched fields

> Template literal remapped keys still enforce value types.

```ds
interface Shape {
    a: number
    b: string
}

type Prefixed<T> = {
    [K in keyof T as K extends string ? `get_${K}` : never]: () => T[K]
};

const bad: Prefixed<Shape> = {
    get_a: () => "no",
    get_b: () => "ok",
};
```

- contains: not assignable
