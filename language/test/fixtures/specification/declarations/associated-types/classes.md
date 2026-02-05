# Associated Types: Classes

Class associated type tests live here.

## classes

### class associated type alias is allowed

> Classes can declare associated type aliases.

```ds
class Box<T> {
    type Item = T;
    value: Item;

    constructor(value: Item) {
        this.value = value;
    }
}

const box = new Box("ok");
box.value satisfies string;
```

### class associated type can declare static parameters

> Class associated type aliases can declare static parameters.

```ds
class Box<T> {
    type Wrap<U> = [T, U];
    value: T;

    constructor(value: T) {
        this.value = value;
    }
}

declare const value: Box<string>.Wrap<int32>;
value satisfies [string, int32];
```

### class associated type constraint rejects incompatible defaults

> Class associated type defaults must satisfy the declared constraint.

```ds
class SizedBox {
    type Item: number = string;
    value: number = 0;
}
```

- contains: not assignable
