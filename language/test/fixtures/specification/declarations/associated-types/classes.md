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

### class generic associated type projection requires static arguments

> Class associated type projections must provide required generic arguments.

```ds
class Box<T> {
    type Wrap<U> = [T, U];
    value: T;

    constructor(value: T) {
        this.value = value;
    }
}

function project<B: Box<string>>(value: B.Wrap): B.Wrap {
    return value;
}
```

- contains: argument

### class associated type constraint rejects incompatible defaults

> Class associated type defaults must satisfy the declared constraint.

```ds
class SizedBox {
    type Item: number = string;
    value: number = 0;
}
```

- contains: not assignable

### class associated type satisfies interface contract

> Classes can provide interface associated types directly in the class body.

```ds
interface Container {
    type Item;
    size(): uint;
}

struct MapEntry<K, V> {
    key: K;
    value: V;
}

class Map<K, V> implements Container {
    type Item = MapEntry<K, V>;
    entries: Item[] = [];

    size(): uint {
        return 0;
    }
}

declare const entry: Map<string, int32>.Item;
entry satisfies MapEntry<string, int32>;
```

### class inherits interface generic associated defaults

> Classes inherit generic associated defaults when no override is provided.

```ds
interface Projected<T> {
    type View<U> = [T, U];
}

class Buffer<T> implements Projected<T> {
    value: T;

    constructor(value: T) {
        this.value = value;
    }
}

declare const value: Buffer<int32>.View<boolean>;
value satisfies [int32, boolean];
```

### class associated type override keeps interface substitution

> Class overrides of interface associated types preserve outer substitutions.

```ds
interface Projected<T> {
    type View<U> = [T, U];
}

class Buffer<T> implements Projected<T> {
    type View<U> = { left: T, right: U };
    value: T;

    constructor(value: T) {
        this.value = value;
    }
}

declare const value: Buffer<int32>.View<boolean>;
value satisfies { left: int32, right: boolean };
```

### class associated type override enforces interface bounds

> Class associated type overrides must satisfy interface bounds.

```ds
interface SizedContainer<T> {
    type Item: T;
}

class BadMap implements SizedContainer<int32> {
    type Item = string;
}
```

- contains: not assignable

### class inherited generic associated type projection requires static arguments

> Class projections must supply required generic arguments inherited from interfaces.

```ds
interface Projected<T> {
    type View<U> = [T, U];
}

class Buffer<T> implements Projected<T> {
    value: T;

    constructor(value: T) {
        this.value = value;
    }
}

function project<B: Buffer<int32>>(value: B.View): B.View {
    return value;
}
```

- contains: argument

### class inherits interface associated type defaults with static value parameters

> Class projections preserve static value arguments inherited from interface defaults.

```ds
interface MatrixLike<T> {
    type Row<comptime n: uint> = [T, n];
}

class Matrix<T> implements MatrixLike<T> {
    value: T;

    constructor(value: T) {
        this.value = value;
    }
}

declare const row: Matrix<float64>.Row<4>;
row satisfies [float64, 4];
```

### class associated defaults can reference sibling associated types

> Class associated type defaults can reference sibling associated types.

```ds
class PairBox<T> {
    type Item = T;
    type Pair<U> = [Item, U];
}

declare const value: PairBox<string>.Pair<int32>;
value satisfies [string, int32];
```

### class associated projections reject extra static arguments

> Class associated type projections reject extra static arguments.

```ds
class PairBox<T> {
    type Item = T;
}

function project<B: PairBox<string>>(value: B.Item<int32>): B.Item<int32> {
    return value;
}
```

- contains: argument

### class inheritance preserves associated type projections

> Subclasses inherit associated type aliases from base classes.

```ds
class Base<T> {
    type Item = T;
}

class Derived<T> extends Base<T> {}

declare const value: Derived<string>.Item;
value satisfies string;
```

### class inheritance allows associated type overrides

> Subclasses can override inherited associated type aliases.

```ds
class Base<T> {
    type Item = T;
}

class Derived<T> extends Base<T> {
    type Item = [T, T];
}

declare const value: Derived<int32>.Item;
value satisfies [int32, int32];
```

### class associated projections work through type aliases

> Associated type projections are preserved through alias indirection.

```ds
class Base<T> {
    type Item = T;
}

type Alias<T> = Base<T>;

declare const value: Alias<boolean>.Item;
value satisfies boolean;
```
