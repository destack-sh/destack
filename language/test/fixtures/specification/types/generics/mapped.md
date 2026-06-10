# Mapped Types

Mapped types transform every key of a shape.

## mapping

### mapped types build object fields

Mapped types produce fields for each key.

```ds
type Flags<T> = { [K in keyof T]: boolean };

interface Person {
    name: string;
    age: number;
}

const ok: Flags<Person> = { name: true, age: false };
```

### mapped types reject incompatible field types

Mapped fields must satisfy the mapped value type.

```ds
type Flags<T> = { [K in keyof T]: boolean };

interface Person {
    name: string;
    age: number;
}

const bad: Flags<Person> = { name: true, age: "no" };
```

- contains: not assignable

### mapped types support optional modifiers

Optional modifiers allow missing fields.

```ds
type Optional<T> = { [K in keyof T]?: T[K] };

interface Person {
    name: string;
    age: number;
}

const ok: Optional<Person> = {};
const ok2: Optional<Person> = { name: "Ada" };
```

### mapped types reject incompatible optional fields

Optional fields still require compatible types.

```ds
type Optional<T> = { [K in keyof T]?: T[K] };

interface Person {
    name: string;
    age: number;
}

const bad: Optional<Person> = { name: "Ada", age: "no" };
```

- contains: not assignable

### mapped types can remove optional modifiers

Optional removal forces required fields.

```ds
type RequiredKeys<T> = { [K in keyof T]-?: T[K] };

interface Person {
    name?: string;
}

const bad: RequiredKeys<Person> = {};
```

- contains: not assignable

### mapped types can remap keys

Key remaps can merge fields into new keys.

```ds
type Renamed<T> = { [K in keyof T as "value"]: T[K] };

interface Person {
    name: string;
    age: number;
}

const ok: Renamed<Person> = { value: "Ada" };
const ok2: Renamed<Person> = { value: 1 };
```

### mapped types reject incompatible remapped values

Remapped fields must still satisfy the mapped value type.

```ds
type Renamed<T> = { [K in keyof T as "value"]: T[K] };

interface Person {
    name: string;
    age: number;
}

const bad: Renamed<Person> = { value: true };
```

- contains: not assignable

### mapped types add readonly modifiers

Readonly modifiers make fields immutable.

```ds
type Frozen<T> = { readonly [K in keyof T]: T[K] };

interface Person {
    name: string;
    age: number;
}

const frozen: Frozen<Person> = { name: "Ada", age: 42 };
const bad: Person = frozen;
```

- contains: not assignable

### mapped types remove readonly modifiers

Removing readonly yields mutable fields.

```ds
type Frozen<T> = { readonly [K in keyof T]: T[K] };
type Mutable<T> = { -readonly [K in keyof T]: T[K] };

interface Person {
    name: string;
    age: number;
}

const ok: Mutable<Person> = { name: "Ada", age: 42 };
```

### mapped types remove optional modifiers

Removing optional yields required fields.

```ds
type Optional<T> = { [K in keyof T]?: T[K] };
type Required<T> = { [K in keyof T]-?: T[K] };

interface Person {
    name: string;
}

const bad: Required<Optional<Person>> = {};
```

- contains: not assignable
