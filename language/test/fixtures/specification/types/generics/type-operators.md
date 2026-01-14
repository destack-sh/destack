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
