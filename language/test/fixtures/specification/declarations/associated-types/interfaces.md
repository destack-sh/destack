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

struct Counter {
    value: int32 = 0;
}

extension of Counter implements Iterable<int32> {
    type Item = int32;

    next(): Item {
        this.value
    }
}

declare const item: Counter.Item;
item satisfies int32;
```

### interface associated type can include a constraint

> Interface associated types can declare a constraint.

```ds
interface SizedIterable {
    type Item: number;
    next(): Item;
}

struct Counter {
    value: int32 = 0;
}

extension of Counter implements SizedIterable {
    type Item = int32;

    next(): Item {
        this.value
    }
}

declare const item: Counter.Item;
item satisfies int32;
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

extension of Counter implements Iterable<int32> {
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

extension<T> of Box<T> implements Wrapper<T> {
    type Item = T;
}

declare const value: Box<int32>.Item;
value satisfies int32;
```

### interface associated type bounds reject incompatible substitutions

> Implementor substitutions that violate associated type bounds are rejected.
> This should fail with the expected contract compatibility diagnostic.

```ds
interface Wrapper<T> {
    type Item: T;
}

struct IntBox {
    value: int32 = 0;
}

extension of IntBox implements Wrapper<int32> {
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

struct Buffer<T> {
    value: T;
}

extension<T> of Buffer<T> implements Slice<T> {
    type View<U> = [T, U];
}

declare const view: Buffer<string>.View<int32>;
view satisfies [string, int32];
```

### interface associated type can use static value parameters

> Interface associated types can declare comptime static value parameters.
> Static value arguments must remain intact across owner substitution and associated projection

```ds
interface Windowed<T> {
    type View<comptime n: uint>;
}

struct MetricWindow {}

extension of MetricWindow implements Windowed<int32> {
    type View<comptime n: uint> = [int32, n];
}

declare const view: MetricWindow.View<4>;
view satisfies [int32, 4];
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

extension of Counter implements Iterable<int32> {
    next(): Item {
        this.value
    }
}

declare const counter: Counter;
counter.next() satisfies int32;
```

### implementors can use inherited associated types without qualification

> Implementor member signatures can reference inherited associated type names directly.
> Inherited contracts should be applied before projection.

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

// inherited contracts should apply before projection
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

// imported class should resolve inherited associated alias from interface default
declare const counter: Counter;
counter.next() satisfies int32;

// projection should resolve after module graph resolution
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

extension of Counter implements Stream<int32> {
    next(): Item {
        this.value
    }
}
```

```ds:main.ds
import { Counter } from "./counter";
import "./impl";

// extension implementation should expose inherited associated alias without qualification
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

extension<T> of Box<T> implements Wrapper<T> {
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
> When there is no override, the inherited default should be used.

```ds
interface Windowed<T> {
    type View<U> = [T, U];
}

struct Buffer<T> {
    value: T;
}

extension<T> of Buffer<T> implements Windowed<T> {}

// inherited contracts should apply before projection
declare const view: Buffer<int32>.View<boolean>;
view satisfies [int32, boolean];
```

### interface associated type rejects incompatible implementations

> Implementor associated types must satisfy interface constraints.
> This should fail with the expected contract compatibility diagnostic.

```ds
interface SizedIterable<T> {
    type Item: number;
    next(): Item;
}

struct Bad {
    value: string = "no";
}

extension of Bad implements SizedIterable<int32> {
    type Item = string;

    next(): Item {
        this.value
    }
}
```

- contains: not assignable

### interface associated type requires matching parameter arity

> Implementor associated types must match the required parameter arity.
> Parameter arity mismatches should be reported on the implementor declaration.

```ds
interface Factory {
    type Item<T>;
}

struct Thing {}

extension of Thing implements Factory {
    type Item = int32;
}
```

- contains: parameter

### interface associated type requires matching parameter kinds

> Implementors must match parameter kinds for associated types.
> Parameter kind mismatches should be reported on the implementor declaration.

```ds
interface Windowed<T> {
    type View<comptime n: uint>;
}

struct Samples {
    value: int32 = 0;
}

extension of Samples implements Windowed<int32> {
    type View<T> = [T, T];
}
```

- contains: parameter

### class implementor associated type requires matching parameter arity

> Class implementors must match associated type parameter arity.
> Parameter arity mismatches should be reported on the implementor declaration.

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
> Parameter kind mismatches should be reported on the implementor declaration.

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

extension of IntGraph implements Graph<int32> {}

declare const node: IntGraph.Node;
node satisfies int32;

declare const edge: IntGraph.Edge<boolean>;
edge satisfies [int32, boolean];
```

### constrained projections preserve interface associated type substitutions

> Functions constrained by an interface can project associated types from implementors.
> Constraint solving should happen before associated projection.

```ds
interface Container<T> {
    type Item = T;
    get(): Item;
}

struct Counter {
    value: int32 = 0;
}

extension of Counter implements Container<int32> {
    get(): Item {
        this.value
    }
}

function read<C: Container<int32>>(container: C): C.Item {
    container.get()
}

declare const counter: Counter;
read(counter) satisfies int32;
```

### associated type projection on constrained type parameters

> Projections are allowed on constrained type parameters.
> Constraint solving should project `I.Item` to the concrete implementor item type.

```ds
interface LocalCursor<T> {
    type Item;
    read(): Item;
}

struct Counter {
    value: int32 = 0;
}

extension of Counter implements LocalCursor<int32> {
    type Item = int32;

    read(): Item {
        this.value
    }
}

function project<I: LocalCursor<int32>>(owner: I): I.Item {
    owner.read()
}

declare const counter: Counter;
project(counter) satisfies Counter.Item;
```

### class implementors can override mixed generic associated defaults

> Class implementors can override mixed type and static value associated defaults.
> When there is no override, the inherited default should be used.

```ds
interface MatrixLike<T> {
    type View<U, comptime n: uint> = [T, U, n];
}

class Matrix<T> implements MatrixLike<T> {
    type View<U, comptime n: uint> = { left: T, right: U, size: n };
}

// inherited contracts should apply before projection
declare const view: Matrix<int32>.View<boolean, 4>;
view satisfies { left: int32, right: boolean, size: 4 };
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
> This should fail with the expected projection shape diagnostic.

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
> This should fail with the expected projection shape diagnostic.

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
> When there is no override, the inherited default should be used.

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

// class owner should project the inherited default from Source<T>
declare const value: Counter.Item;
value satisfies int32;
```

### interface inheritance carries abstract associated requirements

> Interface `extends` chains preserve abstract associated type requirements.
> Abstract requirements stay deferred until a concrete owner provides them.

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

// explicit class implementation should satisfy abstract inherited associated requirement
declare const value: Counter.Item;
value satisfies int32;
```

### interface inheritance rejects missing abstract associated requirements

> Implementors must satisfy abstract associated types inherited through `extends`.
> Concrete owners must provide this declaration.

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
> Type operators should run after specialization, not on unspecialized placeholders.

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

// projected member should reflect substituted operator results
declare const left: Pair<int32, string>.LeftItem;
left satisfies int32;

// projected member should reflect substituted operator results
declare const right: Pair<int32, string>.RightItem;
right satisfies string;
```

### implementing multiple interfaces can share one compatible associated projection

> A single associated alias can satisfy multiple interfaces when requirements agree.

```ds
interface Left<T> {
    type Item = T;
}

interface Right<T> {
    type Item = T;
}

class Shared<T> implements Left<T>, Right<T> {}

declare const value: Shared<int32>.Item;
value satisfies int32;
```

### implementing multiple interfaces rejects incompatible associated defaults

> Implementing interfaces with incompatible associated defaults requires an explicit compatible override.
> This should fail with the expected contract compatibility diagnostic.

```ds
interface Left {
    type Item = int32;
}

interface Right {
    type Item = string;
}

class Broken implements Left, Right {}
```

- contains: associated

### interface abstract associated type must be implemented by classes

> Class implementors must define abstract interface associated types.
> Concrete owners must provide this declaration.

```ds
interface Container<T> {
    type Item;
}

class MissingItem<T> implements Container<T> {}
```

- contains: associated

### interface abstract associated type must be implemented by extensions

> Extension implementors must define abstract interface associated types.
> Concrete owners must provide this declaration.

```ds
interface Container<T> {
    type Item;
}

struct MissingItem<T> {
    value: T;
}

extension<T> of MissingItem<T> implements Container<T> {}
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
> Extra static arguments should be rejected.

```ds
interface Container<T> {
    type Item = T;
}

function project<C: Container<string>>(value: C.Item<int32>): C.Item<int32> {
    return value;
}
```

- contains: argument

### associated type projections resolve through re-export chains

> Associated type projections remain available through type re-exports.

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

// imported implementor should project interface default alias on the class owner
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

// projection should resolve after module graph resolution
declare const value: Box<int32>.Item;
value satisfies int32;
```

- contains: associated

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
> Inherited contracts should be applied before projection.

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

// inherited contracts should apply before projection
declare const row: Matrix.Row<4>;
row satisfies [float64, 4];
```

### interface abstract generic associated type must be implemented by classes

> Class implementors must define abstract generic associated types.
> Concrete owners must provide this declaration.

```ds
interface Factory<T> {
    type Item<U>;
}

class MissingFactory<T> implements Factory<T> {}
```

- contains: associated

### interface abstract generic associated type must be implemented by extensions

> Extension implementors must define abstract generic associated types.
> Concrete owners must provide this declaration.

```ds
interface Factory<T> {
    type Item<U>;
}

struct MissingFactory<T> {
    value: T;
}

extension<T> of MissingFactory<T> implements Factory<T> {}
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

// projection should resolve after module graph resolution
declare const value: Box<int32>.Item<string>;
value satisfies [int32, string];
```

### interface mixed generic associated defaults resolve across module boundaries

> Imported implementors preserve associated defaults that mix type and value static parameters.

```ds:factory.ds
export interface Factory<T> {
    type Item<U, comptime n: uint> = [T, U, n];
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

// mixed type and value substitutions should survive module edges
declare const value: Box<int32>.Item<string, 3>;
value satisfies [int32, string, 3];
```

### interface associated defaults resolve through namespace imports

> Namespace imports preserve interface associated defaults in type positions.
> Namespace access must still resolve to the exported owner symbol before associated projection

```ds:factory.ds
export interface Factory<T> {
    type Item = T;
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
import * as api from "./box";

// projection should resolve after module graph resolution
declare const value: api.Box<int32>.Item;
value satisfies int32;
```

### interface associated defaults can use conditional type operators

> Interface associated defaults evaluate conditional type operators after implementor substitution.
> Type operators should run after specialization, not on unspecialized placeholders.

```ds
interface Select<T> {
    type Item = T extends string ? int32 : int16;
}

class TextSelect implements Select<string> {}
class NumberSelect implements Select<float64> {}

// projected member should reflect substituted operator results
declare const text: TextSelect.Item;
text satisfies int32;

// projected member should reflect substituted operator results
declare const number: NumberSelect.Item;
number satisfies int16;
```

### interface associated defaults can use mapped type operators

> Interface associated defaults evaluate mapped type operators after implementor substitution.
> Type operators should run after specialization, not on unspecialized placeholders.

```ds
interface Project<T> {
    type Shape = { [K in keyof T]: T[K] };
}

class UserProject implements Project<{ id: int32, name: string }> {}

// projected member should reflect substituted operator results
declare const shape: UserProject.Shape;
shape satisfies { id: int32, name: string };
```

### interface associated defaults can compose mapped and conditional operators

> Interface associated defaults can combine mapped operators with conditional value rewriting.
> Substitution should happen before mapped traversal and conditional branch selection.

```ds
interface Normalize<Config> {
    type Shape = { [K in keyof Config]: Config[K] extends boolean ? 1 : Config[K] };
}

class RuntimeConfig implements Normalize<{ enabled: boolean, retries: int32 }> {}

declare const shape: RuntimeConfig.Shape;
shape satisfies { enabled: 1, retries: int32 };
```

### interface associated defaults can project union values after mapped normalization

> Associated defaults can project value unions from mapped intermediate aliases.
> The union projection should observe the already specialized mapped result.

```ds
interface ValueProjection<Row> {
    type Shape = { [K in keyof Row]: Row[K] extends string ? string : Row[K] };
    type Value = Shape[keyof Shape];
}

class UserProjection implements ValueProjection<{ name: string, age: int32 }> {}

declare const value: UserProjection.Value;
value satisfies string | int32;
```

### interface associated defaults are not static expressions for comptime value arguments

> Projected interface associated defaults are not yet valid static value expressions.
> Type projections cannot be consumed as static value expressions in comptime argument positions

```ds
type Bytes<comptime n: number> = uint8[n];

interface BufferShape<T> {
    type Length = T extends string ? 8 : 12;
    type Buffer = Bytes<Length>;
}

class TextBuffer implements BufferShape<string> {}
class NumberBuffer implements BufferShape<float64> {}

declare const short: TextBuffer.Buffer;
short satisfies uint8[8];

declare const wide: NumberBuffer.Buffer;
wide satisfies uint8[12];
```

- static argument must be a static expression