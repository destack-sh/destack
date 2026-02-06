# Associated Types: Interfaces

Interface associated type tests live here.

## interfaces

### interface associated type is allowed

> Interfaces can declare abstract associated types.

```ds
interface Iterable<T> {
    type Item;
    next(): Item;
}
```

### interface associated type can include a constraint

> Interface associated types can declare a constraint.

```ds
interface SizedIterable {
    type Item: number;
    next(): Item;
}
```

### interface associated type is provided by implementors

> Implementors provide concrete associated types.

```ds
interface Iterable<T> {
    type Item;
    next(): Item;
}

struct Counter {
    value: int32 = 0;
}

extension for Counter implements Iterable<int32> {
    type Item = int32;

    next(): Item {
        this.value
    }
}

declare const counter: Counter;
counter.next() satisfies int32;
```

### interface associated type bounds use implementor substitutions

> Interface associated type bounds are checked after applying implementor substitutions.

```ds
interface Wrapper<T> {
    type Item: T;
}

struct Box<T> {
    value: T;
}

extension<T> for Box<T> implements Wrapper<T> {
    type Item = T;
}

declare const value: Box<int32>.Item;
value satisfies int32;
```

### interface associated type bounds reject incompatible substitutions

> Implementor substitutions that violate associated type bounds are rejected.

```ds
interface Wrapper<T> {
    type Item: T;
}

struct IntBox {
    value: int32 = 0;
}

extension for IntBox implements Wrapper<int32> {
    type Item = string;
}
```

- contains: not assignable

### interface associated type can be generic

> Interfaces can declare generic associated types.

```ds
interface Slice<T> {
    type View<U>;
}
```

### interface associated type can use static value parameters

> Interface associated types can declare comptime static value parameters.

```ds
interface Windowed<T> {
    type View<comptime n: uint>;
}
```

### interface associated type default can be used by implementors

> Implementors can rely on interface associated type defaults.

```ds
interface Iterable<T> {
    type Item = T;
    next(): Item;
}

struct Counter {
    value: int32 = 0;
}

extension for Counter implements Iterable<int32> {
    next(): Item {
        this.value
    }
}

declare const counter: Counter;
counter.next() satisfies int32;
```

### implementors can use inherited associated types without qualification

> Implementor member signatures can reference inherited associated type names directly.

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

declare const counter: Counter;
counter.next() satisfies int32;
```

### inherited associated type names resolve across module boundaries

> Implementors resolve inherited associated type names through imported interfaces.

```ds:stream.ds
export interface Stream<T> {
    type Item = T;
    next(): Item;
}
```

```ds:counter.ds
import { Stream } from "./stream";

export class Counter implements Stream<int32> {
    value: int32;

    constructor(value: int32) {
        this.value = value;
    }

    next(): Item {
        this.value
    }
}
```

```ds:main.ds
import { Counter } from "./counter";

declare const counter: Counter;
counter.next() satisfies int32;

declare const item: Counter.Item;
item satisfies int32;
```

### inherited associated type names resolve in cross module extensions

> Extension bodies can use inherited associated type names from imported interfaces.

```ds:stream.ds
export interface Stream<T> {
    type Item = T;
    next(): Item;
}
```

```ds:counter.ds
export struct Counter {
    value: int32 = 0;
}
```

```ds:impl.ds
import { Stream } from "./stream";
import { Counter } from "./counter";

extension for Counter implements Stream<int32> {
    next(): Item {
        this.value
    }
}
```

```ds:main.ds
import { Counter } from "./counter";
import "./impl";

declare const counter: Counter;
counter.next() satisfies int32;
```

### interface associated type default can be overridden

> Implementors can override interface associated type defaults.

```ds
interface Wrapper<T> {
    type Item = T;
}

struct Box<T> {
    value: T;
}

extension<T> for Box<T> implements Wrapper<T> {
    type Item = [T, T];
}

declare const value: Box<int32>.Item;
value satisfies [int32, int32];
```

### interface associated types work with class implementors

> Class implementors satisfy interface associated type contracts.

```ds
interface Container<T> {
    type Item = T;
    get(): Item;
}

class NumberContainer implements Container<int32> {
    type Item = int32;
    value: int32;

    constructor(value: int32) {
        this.value = value;
    }

    get(): Item {
        return this.value;
    }
}

declare const value: NumberContainer.Item;
value satisfies int32;
```

### interface generic associated type defaults are inherited

> Implementors inherit generic associated type defaults when no override is provided.

```ds
interface Windowed<T> {
    type View<U> = [T, U];
}

struct Buffer<T> {
    value: T;
}

extension<T> for Buffer<T> implements Windowed<T> {}

declare const view: Buffer<int32>.View<boolean>;
view satisfies [int32, boolean];
```

### interface associated type rejects incompatible implementations

> Implementor associated types must satisfy interface constraints.

```ds
interface SizedIterable<T> {
    type Item: number;
    next(): Item;
}

struct Bad {
    value: string = "no";
}

extension for Bad implements SizedIterable<int32> {
    type Item = string;

    next(): Item {
        this.value
    }
}
```

- contains: not assignable

### interface associated type requires matching parameter arity

> Implementor associated types must match the required parameter arity.

```ds
interface Factory {
    type Item<T>;
}

struct Thing {}

extension for Thing implements Factory {
    type Item = int32;
}
```

- contains: parameter

### interface associated type requires matching parameter kinds

> Implementors must match parameter kinds for associated types.

```ds
interface Windowed<T> {
    type View<comptime n: uint>;
}

struct Samples {
    value: int32 = 0;
}

extension for Samples implements Windowed<int32> {
    type View<T> = [T, T];
}
```

- contains: parameter

### class implementor associated type requires matching parameter arity

> Class implementors must match associated type parameter arity.

```ds
interface Factory<T> {
    type Item<U>;
}

class BadFactory<T> implements Factory<T> {
    type Item = T;
}
```

- contains: parameter

### class implementor associated type requires matching parameter kinds

> Class implementors must match associated type parameter kinds.

```ds
interface Windowed<T> {
    type View<comptime n: uint>;
}

class BadWindow<T> implements Windowed<T> {
    type View<U> = U;
}
```

- contains: parameter

### interface with multiple associated types supports mixed projections

> Implementors can expose multiple associated type projections from one interface.

```ds
interface Graph<T> {
    type Node = T;
    type Edge<U> = [T, U];
}

struct IntGraph {}

extension for IntGraph implements Graph<int32> {}

declare const node: IntGraph.Node;
node satisfies int32;

declare const edge: IntGraph.Edge<boolean>;
edge satisfies [int32, boolean];
```

### associated type projection on constrained type parameters

> Projections are allowed on constrained type parameters.

```ds
interface Iterable<T> {
    type Item;
}

function head<I: Iterable<int32>>(value: I.Item): I.Item {
    return value;
}
```

### associated type projection supports mixed outer and member substitutions

> Projections preserve both receiver substitutions and associated member substitutions.

```ds
interface Transform<T> {
    type Apply<U> = [T, U];
}

class Mapper<T> implements Transform<T> {}

declare const value: Mapper<int32>.Apply<boolean>;
value satisfies [int32, boolean];
```

### associated type projection requires static value arguments

> Projections must supply required static value arguments.

```ds
interface Windowed<T> {
    type View<comptime n: uint>;
}

function take<W: Windowed<int32>>(value: W.View): W.View {
    return value;
}
```

- contains: argument

### associated type projection requires generic associated arguments

> Projections must supply required generic associated type arguments.

```ds
interface Factory {
    type Item<T>;
}

function project<F: Factory>(value: F.Item): F.Item {
    return value;
}
```

- contains: argument

### interface inheritance carries associated type defaults

> Interface `extends` chains preserve associated type defaults for implementors.

```ds
interface Source<T> {
    type Item = T;
}

interface Stream<T> extends Source<T> {
    next(): Source<T>.Item;
}

class Counter implements Stream<int32> {
    value: int32 = 0;

    next(): Stream<int32>.Item {
        return this.value;
    }
}

declare const value: Counter.Item;
value satisfies int32;
```

### interface inheritance carries abstract associated requirements

> Interface `extends` chains preserve abstract associated type requirements.

```ds
interface Source<T> {
    type Item;
}

interface Stream<T> extends Source<T> {
    next(): Source<T>.Item;
}

class Counter implements Stream<int32> {
    value: int32 = 0;

    type Item = int32;

    next(): Item {
        return this.value;
    }
}

declare const value: Counter.Item;
value satisfies int32;
```

### interface inheritance rejects missing abstract associated requirements

> Implementors must satisfy abstract associated types inherited through `extends`.

```ds
interface Source<T> {
    type Item;
}

interface Stream<T> extends Source<T> {
    next(): Source<T>.Item;
}

class Counter implements Stream<int32> {
    value: int32 = 0;

    next(): int32 {
        return this.value;
    }
}
```

- contains: associated

### implementing multiple interfaces composes associated projections

> Implementing multiple interfaces composes associated type defaults from each interface.

```ds
interface Left<T> {
    type LeftItem = T;
}

interface Right<U> {
    type RightItem = U;
}

class Pair<T, U> implements Left<T>, Right<U> {
    left: T;
    right: U;

    constructor(left: T, right: U) {
        this.left = left;
        this.right = right;
    }
}

declare const left: Pair<int32, string>.LeftItem;
left satisfies int32;

declare const right: Pair<int32, string>.RightItem;
right satisfies string;
```

### interface abstract associated type must be implemented by classes

> Class implementors must define abstract interface associated types.

```ds
interface Container<T> {
    type Item;
}

class MissingItem<T> implements Container<T> {}
```

- contains: associated

### interface abstract associated type must be implemented by extensions

> Extension implementors must define abstract interface associated types.

```ds
interface Container<T> {
    type Item;
}

struct MissingItem<T> {
    value: T;
}

extension<T> for MissingItem<T> implements Container<T> {}
```

- contains: associated

### interface defaults can reference sibling associated types

> Associated type defaults can reuse other associated types on the same interface.

```ds
interface Builder<T> {
    type Item = T;
    type Pair<U> = [Item, U];
}

class StringBuilder implements Builder<string> {}

declare const value: StringBuilder.Pair<int32>;
value satisfies [string, int32];
```

### interface generic defaults can reference sibling generic associated aliases

> Interface generic associated defaults can compose through sibling generic aliases.

```ds
interface Pairing<T> {
    type Entry<V> = { left: T, right: V };
    type Pair<U> = [Entry<U>, Entry<U>];
}

class StringPairing implements Pairing<string> {}

declare const value: StringPairing.Pair<int32>;
value satisfies [{ left: string, right: int32 }, { left: string, right: int32 }];
```

### associated projections reject extra static arguments

> Projections reject extra static arguments for non-generic associated types.

```ds
interface Container<T> {
    type Item = T;
}

function project<C: Container<string>>(value: C.Item<int32>): C.Item<int32> {
    return value;
}
```

- contains: argument

### associated type projections resolve through re export chains

> Associated type projections remain available through type re exports.

```ds:stream.ds
export interface Stream<T> {
    type Item = T;
}
```

```ds:api.ds
export type { Stream } from "./stream";
```

```ds:main.ds
import type { Stream } from "./api";

declare const value: Stream<int32>.Item;
value satisfies int32;
```

### declaration module interfaces provide associated type defaults

> Implementors can inherit associated defaults from declaration module interfaces.

```ds:stream.d.ds
export interface Stream<T> {
    type Item = T;
    next(): Item;
}
```

```ds:counter.ds
import type { Stream } from "./stream";

export class Counter implements Stream<int32> {
    value: int32;

    constructor(value: int32) {
        this.value = value;
    }

    next(): Item {
        this.value
    }
}
```

```ds:main.ds
import { Counter } from "./counter";

declare const counter: Counter;
counter.next() satisfies int32;
```

### class implementors resolve associated projections across module boundaries

> Class implementors expose interface associated projections through imports.

```ds:container.ds
export interface Container<T> {
    type Item = T;
    get(): Item;
}
```

```ds:box.ds
import type { Container } from "./container";

export class Box<T> implements Container<T> {
    value: T;

    constructor(value: T) {
        this.value = value;
    }

    get(): Item {
        this.value
    }
}
```

```ds:main.ds
import { Box } from "./box";

declare const value: Box<int32>.Item;
value satisfies int32;
```

### cross module abstract associated types require explicit class implementations

> Imported interfaces with abstract associated types still require explicit class implementations.

```ds:container.ds
export interface Container<T> {
    type Item;
    get(): Item;
}
```

```ds:box.ds
import type { Container } from "./container";

export class Box<T> implements Container<T> {
    value: T;

    constructor(value: T) {
        this.value = value;
    }

    get(): Item {
        this.value
    }
}
```

```ds:main.ds
import { Box } from "./box";

declare const value: Box<int32>.Item;
value satisfies int32;
```

### nominal interfaces support associated type defaults

> Nominal interfaces can declare associated defaults used by explicit implementors.

```ds
newtype interface Container<T> {
    type Item = T;
    get(): Item;
}

class NumberContainer implements Container<int32> {
    value: int32;

    constructor(value: int32) {
        this.value = value;
    }

    get(): Item {
        this.value
    }
}

declare const value: NumberContainer.Item;
value satisfies int32;
```

### inherited associated projections compose type and value substitutions

> Inherited associated defaults preserve both outer type and inner value substitutions.

```ds
interface MatrixLike<T> {
    type Row<comptime n: uint> = [T, n];
}

interface Renderable<T> extends MatrixLike<T> {
    render(): Row<4>;
}

class Matrix implements Renderable<float64> {
    render(): Row<4> {
        [0.0, 4]
    }
}

declare const row: Matrix.Row<4>;
row satisfies [float64, 4];
```

### interface abstract generic associated type must be implemented by classes

> Class implementors must define abstract generic associated types.

```ds
interface Factory<T> {
    type Item<U>;
}

class MissingFactory<T> implements Factory<T> {}
```

- contains: associated

### interface abstract generic associated type must be implemented by extensions

> Extension implementors must define abstract generic associated types.

```ds
interface Factory<T> {
    type Item<U>;
}

struct MissingFactory<T> {
    value: T;
}

extension<T> for MissingFactory<T> implements Factory<T> {}
```

- contains: associated

### interface generic associated defaults resolve across module boundaries

> Imported implementors preserve generic associated defaults and substitutions.

```ds:factory.ds
export interface Factory<T> {
    type Item<U> = [T, U];
}
```

```ds:box.ds
import type { Factory } from "./factory";

export class Box<T> implements Factory<T> {
    value: T;

    constructor(value: T) {
        this.value = value;
    }
}
```

```ds:main.ds
import { Box } from "./box";

declare const value: Box<int32>.Item<string>;
value satisfies [int32, string];
```
