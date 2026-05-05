# Associated Types

Associated types can project through aliases and accept computed arguments.

## aliases

### associated types pass through aliases

An associated type can refer to another associated type.

```ds
class Segment<Row> {
    type Lane = Row extends string ? [uint8; 8] : [uint8; 4];
}

class Packet<Row> {
    type Part = Segment<Row>;
    type Lane = this.Part.Lane;
}

declare const lane: Packet<string>.Lane;
lane satisfies [uint8; 8];
```

## generics

### generic associated types accept computed values

A generic associated type can receive a computed static argument.

```ds
interface Storage {
    type Page<comptime N: uint>;
}

struct InlineStorage implements Storage {
    type Page<comptime N: uint> = [uint8; N];
}

type DoublePage<S: Storage, comptime N: uint> = S.Page<N * 2>;

declare const page: DoublePage<InlineStorage, 4>;
page satisfies [uint8; 8];
```
