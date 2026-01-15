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

let ctor: CounterCtor = Counter;

let okNext: int32 = ctor.next(1);
let badNext: string = ctor.next(1);
```

- contains: type int32 is not assignable to type string

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
