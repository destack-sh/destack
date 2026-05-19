# Type Alias Declarations

Type alias fixtures cover aliases, generics, object types, mapped types, and conditional types.

## Type Alias Forms

### type alias

Extra whitespace around the type alias should be normalized.

```ds
type   Foo   =   number
```

Type aliases have single spaces around `=` and a trailing semicolon.

```ds expected
type Foo = number;
```

### type alias with union

Union types have spaces around the `|` operator.

```ds
type   Foo   =   string   |   number
```

```ds expected
type Foo = string | number;
```

### type alias with intersection

Intersection types have spaces around the `&` operator.

```ds
type   Foo   =   A   &   B
```

```ds expected
type Foo = A & B;
```

### TypeScript type literal uses semicolons

TypeScript type literals use semicolons between properties.

```ts:main.ts
type Foo = { a: string, b: number }
```

```ts expected
type Foo = { a: string; b: number };
```

### TypeScript type literal keeps semicolons when multiline

Multiline TypeScript type literals include semicolons after each property.

```ts:main.ts line-width=20
type Foo = { a: string, b: number, c: number }
```

```ts expected
type Foo = {
    a: string;
    b: number;
    c: number;
};
```

## Multi-line Type Unions

### long union breaks with leading separators

Long unions break into leading `|` separator lines.

```ds line-width=30
type Result = Success | Failure | Pending | Unknown
```

```ds expected
type Result =
    | Success
    | Failure
    | Pending
    | Unknown;
```

### long intersection breaks at operators

Intersection types also break with operators at the start of lines.
(Unlike in TypeScript, we can't lead with `&` because it's a valid unary operator i.e. references.)

```ds line-width=30
type Combined = HasName & HasAge & HasEmail
```

```ds expected
type Combined = HasName &
    HasAge &
    HasEmail;
```

### long intersection breaks across lines in TypeScript

TypeScript intersections break across lines with trailing `&` when they exceed the width.

```ts:main.ts line-width=30
type Combined = HasName & HasAge & HasEmail
```

```ts expected
type Combined = HasName &
    HasAge &
    HasEmail;
```

### nullable union with object type breaks vertically

Nullable unions with one object-like arm and void-like companions break into a vertical union at narrow widths.

```ds line-width=20
type MaybeUser = { name: string, email: string } | null | undefined
```

```ds expected
type MaybeUser =
    | {
          name: string;
          email: string;
      }
    | null
    | undefined;
```

### nullable union with comment keeps comment on object arm

Comments between members stay attached to the object union arm.

```ds line-width=20
type MaybeUser = { name: string, email: string } /* note */ | null | undefined
```

```ds expected
type MaybeUser =
    | {
          name: string;
          email: string;
      } /* note */
    | null
    | undefined;
```

### intersection with object types expands

Object-like intersection arms keep `&` separators clear when object arms expand.

```ds line-width=30
type WithDetails = { id: string, name: string } & HasMeta & { created: int32 }
```

```ds expected
type WithDetails = {
    id: string;
    name: string;
} & HasMeta & {
        created: int32;
    };
```

## Mapped Types

### mapped type

Mapped types iterate over keys and transform values.

```ds
type Readonly<T> = { [K in keyof T]: T[K] }
```

```ds expected
type Readonly<T> = { [K in keyof T]: T[K] };
```

### mapped type with modifiers

Mapped types support readonly and optional modifiers, including removal.

```ds
type Mutable<T> = { -readonly [K in keyof T]-?: T[K] }
```

```ds expected
type Mutable<T> = { -readonly [K in keyof T]-?: T[K] };
```

### mapped type with explicit add modifiers

Explicit add modifiers are preserved.

```ds
type Explicit<T> = { +readonly [K in keyof T]+?: T[K] }
```

```ds expected
type Explicit<T> = { +readonly [K in keyof T]+?: T[K] };
```

### mapped type with optional modifier

Mapped types can make properties optional or required.

```ds
type Partial<T> = { [K in keyof T]?: T[K] }
```

```ds expected
type Partial<T> = { [K in keyof T]?: T[K] };
```

### mapped type with key remapping

Key remapping uses `as` clause to transform key names.

```ds
type Getters<T> = { [K in keyof T as `get${Capitalize<K>}`]: () => T[K] }
```

```ds expected
type Getters<T> = { [K in keyof T as `get${Capitalize<K>}`]: () => T[K] };
```

### complex mapped type breaks

Long mapped types break to multiple lines.

```ds line-width=40
type DeepReadonly<T> = { readonly [K in keyof T]: DeepReadonly<T[K]> }
```

```ds expected
type DeepReadonly<T> = {
    readonly [K in keyof T]: DeepReadonly<
        T[K]
    >;
};
```

## Conditional Types

### conditional type

Conditional types use `extends` with ternary syntax.

```ds
type IsString<T> = T extends string ? true : false
```

```ds expected
type IsString<T> = T extends string ? true : false;
```

### conditional type with infer

The `infer` keyword extracts types within conditional branches.

```ds
type ReturnType<T> = T extends (...args: any[]) => infer R ? R : never
```

```ds expected
type ReturnType<T> = T extends (...args: any[]) => infer R ? R : never;
```

### conditional type with union distribution

Conditional types distribute over union types.

```ds
type NonNullable<T> = T extends null | undefined ? never : T
```

```ds expected
type NonNullable<T> = T extends null | undefined ? never : T;
```

## Type Operators

### keyof operator

The `keyof` operator extracts keys from a type.

```ds
type Keys<T> = keyof T
```

```ds expected
type Keys<T> = keyof T;
```

### typeof operator

The `typeof` operator gets the type of a value.

```ds
type Config = typeof defaultConfig
```

```ds expected
type Config = typeof defaultConfig;
```

### indexed access type

Indexed access types retrieve property types.

```ds
type NameType = Person["name"]
```

```ds expected
type NameType = Person["name"];
```

### template literal type

Template literal types create string literal unions.
Destack uses `: Type` for type parameter constraints, not `extends Type` like TypeScript.

```ds
type EventName<T: string> = `on${Capitalize<T>}`
```

```ds expected
type EventName<T: string> = `on${Capitalize<T>}`;
```
