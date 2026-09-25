# Type Alias Declarations

Type alias fixtures cover aliases, generics, object types, mapped types, and conditional types.

## Type Alias Forms

### type alias

Extra whitespace around the type alias should be normalized.

```tspp
type   Foo   =   number
```

Type aliases have single spaces around `=` and a trailing semicolon.

```tspp expected
type Foo = number;
```

### type alias with union

Union types have spaces around the `|` operator.

```tspp
type   Foo   =   string   |   number
```

```tspp expected
type Foo = string | number;
```

### type alias with intersection

Intersection types have spaces around the `&` operator.

```tspp
type   Foo   =   A   &   B
```

```tspp expected
type Foo = A & B;
```

### type literal uses semicolons

Type literals use semicolons between properties.

```tspp:main.tspp
type Foo = { a: string, b: number }
```

```tspp expected
type Foo = { a: string; b: number };
```

### multiline type literal keeps semicolons

Multiline type literals include semicolons after each property.

```tspp:main.tspp line-width=20
type Foo = { a: string, b: number, c: number }
```

```tspp expected
type Foo = {
    a: string;
    b: number;
    c: number;
};
```

## Multi-line Type Unions

### long union breaks with leading separators

Long unions break into leading `|` separator lines.

```tspp line-width=30
type Result = Success | Failure | Pending | Unknown
```

```tspp expected
type Result =
    | Success
    | Failure
    | Pending
    | Unknown;
```

### long intersection breaks at operators

Intersection types also break with operators at the start of lines.
(Unlike in TypeScript, we can't lead with `&` because it's a valid unary operator i.e. references.)

```tspp line-width=30
type Combined = HasName & HasAge & HasEmail
```

```tspp expected
type Combined = HasName &
    HasAge &
    HasEmail;
```

### long intersection breaks across lines

Intersections break across lines with trailing `&` when they exceed the width.

```tspp:main.tspp line-width=30
type Combined = HasName & HasAge & HasEmail
```

```tspp expected
type Combined = HasName &
    HasAge &
    HasEmail;
```

### nullable union with object type breaks vertically

Nullable unions with one object-like arm and void-like companions break into a vertical union at narrow widths.

```tspp line-width=20
type MaybeUser = { name: string, email: string } | null | undefined
```

```tspp expected
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

```tspp line-width=20
type MaybeUser = { name: string, email: string } /* note */ | null | undefined
```

```tspp expected
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

```tspp line-width=30
type WithDetails = { id: string, name: string } & HasMeta & { created: int32 }
```

```tspp expected
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

```tspp
type Readonly<T> = { [K in keyof T]: T[K] }
```

```tspp expected
type Readonly<T> = { [K in keyof T]: T[K] };
```

### mapped type with modifiers

Mapped types support readonly and optional modifiers, including removal.

```tspp
type Mutable<T> = { -readonly [K in keyof T]-?: T[K] }
```

```tspp expected
type Mutable<T> = { -readonly [K in keyof T]-?: T[K] };
```

### mapped type with explicit add modifiers

Explicit add modifiers are preserved.

```tspp
type Explicit<T> = { +readonly [K in keyof T]+?: T[K] }
```

```tspp expected
type Explicit<T> = { +readonly [K in keyof T]+?: T[K] };
```

### mapped type with optional modifier

Mapped types can make properties optional or required.

```tspp
type Partial<T> = { [K in keyof T]?: T[K] }
```

```tspp expected
type Partial<T> = { [K in keyof T]?: T[K] };
```

### mapped type with key remapping

Key remapping uses `as` clause to transform key names.

```tspp
type Getters<T> = { [K in keyof T as `get${Capitalize<K>}`]: () => T[K] }
```

```tspp expected
type Getters<T> = { [K in keyof T as `get${Capitalize<K>}`]: () => T[K] };
```

### complex mapped type breaks

Long mapped types break to multiple lines.

```tspp line-width=40
type DeepReadonly<T> = { readonly [K in keyof T]: DeepReadonly<T[K]> }
```

```tspp expected
type DeepReadonly<T> = {
    readonly [K in keyof T]: DeepReadonly<
        T[K]
    >;
};
```

## Conditional Types

### conditional type

Conditional types use `extends` with ternary syntax.

```tspp
type IsString<T> = T extends string ? true : false
```

```tspp expected
type IsString<T> = T extends string ? true : false;
```

### conditional type with infer

The `infer` keyword extracts types within conditional branches.

```tspp
type ReturnType<T> = T extends (...args: any[]) => infer R ? R : never
```

```tspp expected
type ReturnType<T> = T extends (...args: any[]) => infer R ? R : never;
```

### conditional type with union distribution

Conditional types distribute over union types.

```tspp
type NonNullable<T> = T extends null | undefined ? never : T
```

```tspp expected
type NonNullable<T> = T extends null | undefined ? never : T;
```

## Type Operators

### keyof operator

The `keyof` operator extracts keys from a type.

```tspp
type Keys<T> = keyof T
```

```tspp expected
type Keys<T> = keyof T;
```

### typeof operator

The `typeof` operator gets the type of a value.

```tspp
type Config = typeof defaultConfig
```

```tspp expected
type Config = typeof defaultConfig;
```

### indexed access type

Indexed access types retrieve property types.

```tspp
type NameType = Person["name"]
```

```tspp expected
type NameType = Person["name"];
```

### template literal type

Template literal types create string literal unions.
TS++ uses `: Type` for type parameter constraints, not `extends Type` like TypeScript.

```tspp
type EventName<T: string> = `on${Capitalize<T>}`
```

```tspp expected
type EventName<T: string> = `on${Capitalize<T>}`;
```
