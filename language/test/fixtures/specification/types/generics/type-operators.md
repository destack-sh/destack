# Type Operators

## keyof builds literal key unions

> `keyof` produces a union of literal keys.

```ds
interface Person {
    name: string
    age: number
}

type Keys = keyof Person;

const name: Keys = "name";
const age: Keys = "age";
const bad: Keys = "title";
```

- contains: not assignable

## in returns true for existing keys

> `in` checks whether a key type is assignable to `keyof`.

```ds
interface Person {
    name: string
    age: number
}

type HasName = "name" in Person;

const ok: HasName = true;
const bad: HasName = false;
```

- contains: not assignable

## in returns false for missing keys

> Missing keys result in `false`.

```ds
interface Person {
    name: string
    age: number
}

type HasTitle = "title" in Person;

const ok: HasTitle = false;
const bad: HasTitle = true;
```

- contains: not assignable

## indexed access returns property types

> Indexed access resolves to the property type.

```ds
interface Person {
    name: string
}

type Name = Person["name"];

const ok: Name = "Ada";
const bad: Name = 42;
```

- contains: not assignable

## readonly arrays accept mutable arrays

> Mutable arrays are assignable to readonly arrays.

```ds
declare let values: number[];
let frozen: readonly number[] = values;
frozen satisfies readonly number[];
```

## readonly arrays reject mutable assignment

> Readonly arrays are not assignable to mutable arrays.

```ds
declare let frozen: readonly number[];
let bad: number[] = frozen;
```

- contains: not assignable

## readonly tuples accept mutable tuples

> Mutable tuples are assignable to readonly tuples.

```ds
type Pair = (number, string);
type ReadonlyPair = readonly (number, string);

declare let pair: Pair;
let frozen: ReadonlyPair = pair;
frozen satisfies ReadonlyPair;
```

## readonly tuples reject mutable assignment

> Readonly tuples are not assignable to mutable tuples.

```ds
type Pair = (number, string);
type ReadonlyPair = readonly (number, string);

declare let frozen: ReadonlyPair;
let bad: Pair = frozen;
```

- contains: not assignable

## readonly tuple elements do not imply readonly tuples

> Tuple element modifiers do not make the tuple readonly.

```ds
type ElemReadonly = (readonly int32, int32);

declare let values: ElemReadonly;
let arrayOk: int32[] = values;
arrayOk satisfies int32[];
```

## readonly tuple elements reject mutable element assignment

> Readonly tuple elements are not assignable to mutable tuple elements.

```ds
type ElemReadonly = (readonly int32, int32);
type Mutable = (int32, int32);

declare let values: ElemReadonly;
let bad: Mutable = values;
```

- contains: not assignable

## readonly tuples reject mutable array assignment

> Readonly tuples are not assignable to mutable arrays.

```ds
type ReadonlyPair = readonly (int32, int32);

declare let frozen: ReadonlyPair;
let bad: int32[] = frozen;
```

- contains: not assignable

## extends returns true for assignable types

> `extends` returns `true` when the left type is assignable to the right type.

```ds
type IsNumber = int32 extends number;

const ok: IsNumber = true;
const bad: IsNumber = false;
```

- contains: not assignable

## extends returns false for non assignable types

> `extends` returns `false` when the left type is not assignable to the right type.

```ds
type IsString = string extends int32;

const ok: IsString = false;
const bad: IsString = true;
```

- contains: not assignable

## implements returns true for compatible types

> `implements` returns `true` when the left type implements the right type.

```ds
interface Drawable {
    draw(): void
}

struct DrawnPoint implements Drawable {
    x: int32

    draw(): void {}
}

type IsDrawable = DrawnPoint implements Drawable;

const ok: IsDrawable = true;
const bad: IsDrawable = false;
```

- contains: not assignable

## implements returns false for incompatible types

> `implements` returns `false` when the left type does not satisfy the right type.

```ds
interface Drawable {
    draw(): void
}

struct PlainPoint {
    x: int32
}

type IsDrawable = PlainPoint implements Drawable;

const ok: IsDrawable = false;
const bad: IsDrawable = true;
```

- contains: not assignable

## conditional types pick true branch

> Conditional types select the matching branch.

```ds
type Select<T> = T extends string ? string : int32;

let ok: Select<string> = "ok";
let bad: Select<string> = 1;
ok satisfies string;
```

- contains: type int32 is not assignable to type string

## conditional types pick false branch

> Conditional types select the else branch when the match fails.

```ds
type Select<T> = T extends string ? string : int32;

let ok: Select<int32> = 1;
let bad: Select<int32> = "no";
ok satisfies int32;
```

- contains: type string is not assignable to type int32

## conditional types distribute over unions

> Conditional types distribute over union inputs.

```ds
type OnlyStrings<T> = T extends string ? T : never;

let ok: OnlyStrings<string | int32> = "ok";
let bad: OnlyStrings<string | int32> = 1;
ok satisfies string;
```

- contains: type int32 is not assignable to type string

## mapped types build object fields

> Mapped types produce fields for each key.

```ds
type Flags<T> = { [K in keyof T]: boolean };

interface Person {
    name: string
    age: number
}

const ok: Flags<Person> = { name: true, age: false };
const bad: Flags<Person> = { name: true, age: "no" };
```

- contains: type string is not assignable to type boolean

## mapped types support optional modifiers

> Optional modifiers allow missing fields.

```ds
type Optional<T> = { [K in keyof T]?: T[K] };

interface Person {
    name: string
    age: number
}

const ok: Optional<Person> = {};
const ok2: Optional<Person> = { name: "Ada" };
const bad: Optional<Person> = { name: "Ada", age: "no" };
```

- contains: type string is not assignable to type number

## mapped types can remove optional modifiers

> Optional removal forces required fields.

```ds
type RequiredKeys<T> = { [K in keyof T]-?: T[K] };

interface Person {
    name?: string
}

const bad: RequiredKeys<Person> = {};
```

- contains: not assignable

## mapped types can remap keys

> Key remaps can merge fields into new keys.

```ds
type Renamed<T> = { [K in keyof T as "value"]: T[K] };

interface Person {
    name: string
    age: number
}

const ok: Renamed<Person> = { value: "Ada" };
const ok2: Renamed<Person> = { value: 1 };
const bad: Renamed<Person> = { value: true };
```

- contains: type true is not assignable to type string | int32

## typeof returns value types for locals

> `typeof` returns the value type of a local binding.

```ds
const value = 42;

type ValueType = typeof value;

let ok: ValueType = 42;
let bad: ValueType = "no";
```

- contains: type string is not assignable to type int32

## typeof returns constructor types for classes

> `typeof` on a class returns the constructor value type with static members.

```ds
class Counter {
    static version: int32
    value: int32

    constructor(value: int32) {}
}

type CounterCtor = typeof Counter;

declare function takesCounter(ctor: { new(value: int32): Counter }): void;

takesCounter(Counter);

let okVersion: CounterCtor["version"] = 1;
let badVersion: CounterCtor["version"] = "no";
```

- contains: type string is not assignable to type int32

### typeof includes static methods

> `typeof` exposes static methods on the constructor value type.

```ds
class Counter {
    static next(value: int32): int32 { return value + 1 }
}

type CounterCtor = typeof Counter;
type CounterNext = (typeof Counter)["next"];

let ctor: CounterCtor = Counter;
let okNext: int32 = ctor.next(1);

let okFn: CounterNext = Counter.next;
```

### typeof rejects static method call assignability

> `typeof` static method results must match the expected type.

```ds
class Counter {
    static next(value: int32): int32 { return value + 1 }
}

type CounterCtor = typeof Counter;

let ctor: CounterCtor = Counter;
let badNext: string = ctor.next(1);
```

- contains: type int32 is not assignable to type string

### typeof rejects incompatible static method types

> `typeof` indexed access must preserve the static method signature.

```ds
class Counter {
    static next(value: int32): int32 { return value + 1 }
}

type CounterNext = (typeof Counter)["next"];

let badFn: CounterNext = (value: string) => value;
```

- contains: type (string): string is not assignable to type CounterNext

### typeof inherits base constructors

> Classes without constructors inherit the base constructor signature.

```ds
class Base {
    constructor(value: int32) {}
}

class Child extends Base {}

declare function takesChild(ctor: { new(value: int32): Child }): void;

takesChild(Child);
```

## typeof returns constructor types for structs

> `typeof` on a struct returns the constructor value type with static members.

```ds
struct Point {
    static tag: string
    x: int32
    y: int32
}

type PointCtor = typeof Point;

let okTag: PointCtor["tag"] = "ok";
let badTag: PointCtor["tag"] = 1;
```

- contains: type int32 is not assignable to type string
