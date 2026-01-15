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
