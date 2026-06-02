# Object Literals

Object literal type inference and checking.

## objects

### object with properties

Objects infer property types from their values.

```ds
const x = { a: 1, b: "two" };
x satisfies { a: number; b: string };
```

### object with shorthand properties

Shorthand fields use the binding type.

```ds
const name = "Ada";
const age = 42;
const person = { name, age };
person satisfies { name: string; age: number };
```

### empty object

Empty objects have no properties.

```ds
const x = {};
x satisfies {};
```

### nested object

Objects can contain nested objects.

```ds
const x = { a: { b: 1 } };
x satisfies { a: { b: number } };
```

### object with const assertion

Const assertions preserve literal property types.

```ds
const x = { a: 1, b: "two" } as const;
x satisfies { readonly a: 1; readonly b: "two" };
```

## struct literals

### inferred struct literal type

Struct literals may use `_` when the contextual type determines the nominal struct.

```ds
struct Point {
    x: int32;
}

let point: Point = _ { x: 1 };

point satisfies Point;
```

### struct update spread

Struct expressions can update an existing struct value through spread.

```ds
struct Point {
    x: int32;
    y: int32;
}

const point = Point { x: 1, y: 2 };
const moved = Point { ...point, x: 3 };

moved satisfies Point;
```

### object spread from struct

Object literals can spread struct fields into a structural object.

```ds
struct Point {
    x: int32;
    y: int32;
}

const point = Point { x: 1, y: 2 };
const object = { ...point, label: "origin" };

object satisfies { x: int32; y: int32; label: string };
```

### struct spread from object

Struct expressions can spread structural object fields into a nominal struct.

```ds
struct Point {
    x: int32;
    y: int32;
}

const base = { x: 1, y: 2 };
const point: Point = _ { ...base };

point satisfies Point;
```

## object spreads

### object spread adds fields

Object literals incorporate spread fields.

```ds
const base = { a: 1, b: "two" };
const value = { ...base, c: true };
value satisfies { a: number; b: string; c: boolean };
```

### object spread overrides fields

Later fields override earlier spread fields.

```ds
const base = { a: 1, b: 2 };
const value = { ...base, b: "two" };
value satisfies { a: number; b: string };
```

### object spread preserves unions

Union spreads produce union object shapes.

```ds
const value: { a: number } | { b: string } = { a: 1 };
const merged = { ...value };
merged satisfies { a: number } | { b: string };
```

### object spread unwraps aliases

Object spreads unwrap structural type aliases.

```ds
type Base = { a: number };
const base: Base = { a: 1 };
const value = { ...base, b: "two" };
value satisfies { a: number; b: string };
```

### object spread merges intersections

Object spreads merge intersection shapes.

```ds
const base: { a: number } & { b: string } = { a: 1, b: "two" };
const value = { ...base, c: true };
value satisfies { a: number; b: string; c: boolean };
```

## contextual objects

### contextual object literal

Object literals use contextual types for property inference.

```ds
const value: { a: number; b: string } = { a: 1, b: "hi" };
value satisfies { a: number; b: string };
```

### contextual object literal mismatch

Object literal properties must satisfy contextual field types.

```ds
const value: { a: number; b: string } = { a: 1, b: 2 };
```

- contains: not assignable

### contextual object literal via alias

Contextual object types are preserved through aliases.

```ds
type Point = { x: number; y: number };

const value: Point = { x: 1, y: 2 };
value satisfies { x: number; y: number };
```

### contextual object literal alias mismatch

Alias contextual types still enforce property constraints.

```ds
type Point = { x: number; y: number };

const value: Point = { x: 1, y: "hi" };
```

- contains: not assignable

### contextual object spread literal

Object spreads still respect contextual object types.

```ds
const value: { a: number } = { ...{ a: 1 } };
value satisfies { a: number };
```

### contextual object spread literal mismatch

Object spreads reject fields that violate contextual types.

```ds
const value: { a: number } = { ...{ a: "hi" } };
```

- contains: not assignable to type { a: number }

## object members

### object toString resolves

Object literals expose standard object members.


```ds
const value = { a: 1, b: "two" };
const text = value.toString();
text satisfies string;
```

### object hasOwnProperty resolves

Object literals expose hasOwnProperty.


```ds
const value = { a: 1 };
const result = value.hasOwnProperty("a");
result satisfies boolean;
```
