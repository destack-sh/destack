# Associated Types: Basics

Small associated type fixtures to introduce core behavior.

## basic

### struct owner can project a basic associated type

> Struct owners can declare associated type aliases.
> Projecting that alias from a concrete owner should yield the declared type.

```ds
struct Box<T> {
    type Item = T;
    value: Item;
}

const value: Box<string>.Item = "ok";
value satisfies string;
```

### class implementor can inherit an interface associated default

> Interface associated defaults should flow into class implementors.
> Projecting through the implementor should use the inherited default.

```ds
interface Stream<T> {
    type Item = T;
    next(): Item;
}

class Counter implements Stream<int32> {
    value: int32;

    constructor(value: int32) {
        this.value = value;
    }

    next(): Item {
        this.value
    }
}

declare const item: Counter.Item;
item satisfies int32;
```

### class owner can project generic associated aliases

> Class owners can declare generic associated aliases.
> Outer substitutions and member substitutions should both apply.

```ds
class Matrix<T> {
    type Row<U> = [T, U];
}

declare const row: Matrix<boolean>.Row<int32>;
row satisfies [boolean, int32];
```

### associated type projections stay in type space

> Associated types are type-only members.
> Runtime member access should not treat associated types as values.

```ds
class Packet {
    type Size = uint32;
}

const size = Packet.Size;
```

- contains: does not exist
