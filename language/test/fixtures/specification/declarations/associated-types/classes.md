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

### class associated type can declare static value parameters

> Class associated type aliases can declare comptime static value parameters.

```ds
class Matrix<T> {
    type Row<comptime n: uint> = [T, n];
}

declare const row: Matrix<float64>.Row<4>;
row satisfies [float64, 4];
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

### class associated interface implementations resolve across module boundaries

> Class associated aliases satisfy interface contracts across imports.

```ds:container.ds
export interface Container<V> {
    type Item;
    get(): Item;
}
```

```ds:entry.ds
export struct MapEntry<K, V> {
    key: K;
    value: V;
}
```

```ds:map.ds
import { Container } from "./container";
import { MapEntry } from "./entry";

export class Map<K, V> implements Container<V> {
    type Item = MapEntry<K, V>;
    entry: Item;

    constructor(entry: Item) {
        this.entry = entry;
    }

    get(): Item {
        this.entry
    }
}
```

```ds:main.ds
import { Map } from "./map";
import { MapEntry } from "./entry";

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

### class generic associated defaults can reference sibling generic aliases

> Class generic associated aliases can compose through sibling generic aliases.

```ds
class PairBox<T> {
    type Entry<V> = { left: T, right: V };
    type Pair<U> = [Entry<U>, Entry<U>];
}

declare const value: PairBox<string>.Pair<int32>;
value satisfies [{ left: string, right: int32 }, { left: string, right: int32 }];
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

### class associated projections resolve across module boundaries

> Class associated type projections are available through imported modules.

```ds:box.ds
export class Box<T> {
    type Item = T;
}
```

```ds:main.ds
import { Box } from "./box";

declare const value: Box<int32>.Item;
value satisfies int32;
```

### class generic associated projections resolve across module boundaries

> Imported class projections preserve outer and member substitutions.

```ds:box.ds
export class Box<T> {
    type Wrap<U> = [T, U];
}
```

```ds:main.ds
import { Box } from "./box";

declare const value: Box<string>.Wrap<int32>;
value satisfies [string, int32];
```

### class associated projections work on constrained generic parameters

> Generic constraints can project class associated types in signatures.

```ds
class Box<T> {
    type Item = T;
    value: Item;

    constructor(value: Item) {
        this.value = value;
    }
}

function cloneValue<B: Box<int32>>(value: B.Item): B.Item {
    value
}

cloneValue(1) satisfies int32;
```

### class mixed generic associated projections resolve across module boundaries

> Imported class projections preserve mixed type and value substitutions.

```ds:box.ds
export class Box<T> {
    type Wrap<U, comptime n: uint> = [T, U, n];
}
```

```ds:main.ds
import { Box } from "./box";

declare const value: Box<string>.Wrap<int32, 3>;
value satisfies [string, int32, 3];
```

### class inheritance preserves generic associated defaults

> Subclasses inherit generic associated defaults from base classes.

```ds
class Base<T> {
    type View<U> = [T, U];
}

class Derived<T> extends Base<T> {}

declare const value: Derived<float64>.View<boolean>;
value satisfies [float64, boolean];
```

### class inheritance override projections resolve across module boundaries

> Imported subclasses preserve associated type overrides through base class inheritance.

```ds:base.ds
export class Base<T> {
    type Item = T;
}
```

```ds:derived.ds
import { Base } from "./base";

export class Derived<T> extends Base<T> {
    type Item = [T, T];
}
```

```ds:main.ds
import { Derived } from "./derived";

declare const item: Derived<int32>.Item;
item satisfies [int32, int32];
```

### class inheritance preserves associated type defaults with value parameters

> Subclasses inherit associated defaults with static value parameters.

```ds
class MatrixLike<T> {
    type Row<comptime n: uint> = [T, n];
}

class Matrix<T> extends MatrixLike<T> {}

declare const row: Matrix<int32>.Row<4>;
row satisfies [int32, 4];
```

### abstract classes can declare abstract associated types

> Concrete subclasses can satisfy abstract associated type requirements.

```ds
abstract class Base<T> {
    type Item;
    abstract get(): Item;
}

class Derived extends Base<int32> {
    type Item = int32;

    override
    get(): Item {
        1
    }
}

const value = new Derived().get();
value satisfies int32;
```

### abstract classes can declare abstract generic associated types

> Concrete subclasses can satisfy abstract generic associated requirements.

```ds
abstract class Base {
    type Wrap<U>;
}

class Derived extends Base {
    type Wrap<U> = [int32, U];
}

declare const value: Derived.Wrap<string>;
value satisfies [int32, string];
```

### concrete subclasses must implement inherited abstract associated types

> Concrete subclasses must implement abstract associated aliases from base classes.

```ds
abstract class Base<T> {
    type Item;
}

class Derived extends Base<int32> {}
```

- contains: missing associated type implementation

### concrete subclasses must implement inherited abstract generic associated types

> Concrete subclasses must implement abstract generic associated aliases from base classes.

```ds
abstract class Base {
    type Wrap<U>;
}

class Derived extends Base {}
```

- contains: missing associated type implementation

### abstract subclasses can defer inherited abstract associated types

> Abstract subclasses can defer abstract associated aliases to concrete subclasses.

```ds
abstract class Base<T> {
    type Item;
}

abstract class Mid<T> extends Base<T> {}

class Leaf extends Mid<int32> {
    type Item = int32;
}

declare const value: Leaf.Item;
value satisfies int32;
```

### abstract classes can defer interface associated requirements

> Abstract classes can defer interface associated aliases to concrete subclasses.

```ds
interface Container {
    type Item;
    get(): Item;
}

abstract class Base implements Container {
    abstract get(): Item;
}

class Box extends Base {
    type Item = int32;

    override
    get(): Item {
        1
    }
}

const value = new Box().get();
value satisfies int32;
```

### abstract associated requirements flow across module boundaries

> Imported concrete subclasses must satisfy inherited abstract associated aliases.

```ds:base.ds
export abstract class Base<T> {
    type Item;
}
```

```ds:derived.ds
import { Base } from "./base";

export class Derived extends Base<int32> {}
```

```ds:main.ds
import { Derived } from "./derived";

const value = new Derived();
value satisfies Derived;
```

- contains: missing associated type implementation

### abstract associated requirements can be deferred across module boundaries

> Imported abstract subclasses can defer abstract associated aliases to concrete leaves.

```ds:base.ds
export abstract class Base<T> {
    type Item;
}
```

```ds:mid.ds
import { Base } from "./base";

export abstract class Mid<T> extends Base<T> {}
```

```ds:leaf.ds
import { Mid } from "./mid";

export class Leaf extends Mid<int32> {
    type Item = int32;
}
```

```ds:main.ds
import { Leaf } from "./leaf";

declare const value: Leaf.Item;
value satisfies int32;
```

### class associated projections resolve through namespace imports

> Namespace imports preserve class associated projections in type positions.

```ds:box.ds
export class Box<T> {
    type Item = T;
    value: Item;

    constructor(value: Item) {
        this.value = value;
    }
}
```

```ds:main.ds
import * as models from "./box";

declare const value: models.Box<string>.Item;
value satisfies string;
```

### class associated types are not runtime members

> Associated type aliases are type only and cannot be accessed as runtime values.

```ds
class Box<T> {
    type Item = T;
}

const value = Box.Item;
```

- contains: does not exist

### class associated types can use conditional type operators

> Class associated type aliases can evaluate conditional type operators with outer substitutions.

```ds
class Box<T> {
    type Item = T extends string ? int32 : int16;
}

declare const text: Box<string>.Item;
text satisfies int32;

declare const flag: Box<boolean>.Item;
flag satisfies int16;
```

### class associated types can use mapped type operators

> Class associated type aliases can evaluate mapped type operators over outer substitutions.

```ds
class Project<T> {
    type Shape = { [K in keyof T]: T[K] };
}

declare const shape: Project<{ left: int32, right: string }>.Shape;
shape satisfies { left: int32, right: string };
```

### class associated types are not static expressions for comptime value arguments

> Projected associated types are not yet valid static value expressions.

```ds
type Bytes<comptime n: number> = uint8[n];

class BufferShape<T> {
    type Length = T extends string ? 8 : 12;
    type Buffer = Bytes<Length>;
}

declare const short: BufferShape<string>.Buffer;
short satisfies uint8[8];

declare const wide: BufferShape<boolean>.Buffer;
wide satisfies uint8[12];
```

- contains: static argument must be a static expression

### class conditional associated projections resolve across module boundaries

> Imported class associated type projections preserve conditional substitutions.

```ds:box.ds
export class Box<T> {
    type Item = T extends string ? int32 : int16;
}
```

```ds:main.ds
import { Box } from "./box";

declare const text: Box<string>.Item;
text satisfies int32;

declare const count: Box<float64>.Item;
count satisfies int16;
```

### class associated projections remain non static expressions across module boundaries

> Imported associated projections are not yet valid static value expressions.

```ds:box.ds
export type Bytes<comptime n: number> = uint8[n];

export class BufferShape<T> {
    type Length = T extends string ? 8 : 12;
    type Buffer = Bytes<Length>;
}
```

```ds:main.ds
import { BufferShape } from "./box";

declare const short: BufferShape<string>.Buffer;
short satisfies uint8[8];

declare const wide: BufferShape<float64>.Buffer;
wide satisfies uint8[12];
```

- contains: static argument must be a static expression
