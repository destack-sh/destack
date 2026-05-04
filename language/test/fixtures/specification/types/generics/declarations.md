# Declarations

Generic declarations may bind types and static values.

## type aliases

### explicit type arguments on type aliases

> Type aliases accept explicit type arguments.

```ds
type Box<T> = { value: T }

declare function makeBox(): Box<number>;

let value: Box<number> = makeBox();
value satisfies Box<number>;
```

### type argument mismatch on type aliases

> Type arguments must satisfy declared bounds.

```ds
type Box<T: number> = { value: T }

declare function makeBox(): Box<number>;

let value: Box<string> = makeBox();
```

- type string is not assignable to type number

### comptime value arguments on type aliases

> Comptime value arguments are checked against declared types.

```ds
type Buffer<T, comptime N: number> = { value: T }

declare function makeBuffer(): Buffer<string, 4>;

let buffer: Buffer<string, 4> = makeBuffer();
buffer satisfies Buffer<string, 4>;
```

### comptime value arguments require static expressions

> Comptime value arguments must be static expressions.

```ds
type Buffer<T, comptime N: number> = { value: T }

declare function makeBuffer(): Buffer<string, 4>;

let buffer: Buffer<string, comptime 4> = makeBuffer();
```

- contains: static expression

### comptime value argument mismatch on type aliases

> Comptime value arguments must satisfy declared types.

```ds
type Buffer<T, comptime N: number> = { value: T }

declare function makeBuffer(): Buffer<string, 4>;

let buffer: Buffer<string, true> = makeBuffer();
```

- type true is not assignable to type number

### default type parameters on type aliases

> Type parameters fall back to defaults when omitted.

```ds
type Box<T = number> = { value: T }

declare function makeBox(): Box;

let value: Box = makeBox();
value satisfies Box<number>;
```

### default comptime value arguments on type aliases

> Comptime value arguments fall back to defaults when omitted.

```ds
type Buffer<T, comptime N: number = 4> = { value: T }

declare function makeBuffer(): Buffer<string>;

let buffer: Buffer<string> = makeBuffer();
buffer satisfies Buffer<string, 4>;
```

### default comptime value arguments require static expressions

> Comptime value defaults must be static expressions.

```ds
type Buffer<T, comptime N: number = comptime 4> = { value: T }

declare function makeBuffer(): Buffer<string>;

let buffer: Buffer<string> = makeBuffer();
```

- contains: static expression

### tuple comptime arguments stay grouped for direct aliases

> Tuple comptime arguments stay grouped when passed through aliases.

```ds
type And<Types extends boolean[]> = Types[number] extends true ? true : false;
type MutuallyExtends<Left, Right> = And<
  [Left extends Right ? true : false, Right extends Left ? true : false]
>;

type Result = MutuallyExtends<number, number>;

declare let value: Result;
value satisfies true;
```

### tuple comptime arguments stay grouped for tuple types

> Tuple type arguments remain grouped for direct aliases.

```ds
type Wrap<T> = T;
type Alias = Wrap<(number, string)>;

declare let value: Alias;
value satisfies (number, string);
```

### tuple comptime arguments stay grouped for imported aliases

> Tuple comptime arguments stay grouped across imported aliases.

```ts:utils.d.ts
export type And<Types extends boolean[]> = Types[number] extends true ? true : false;
```

```ts:branding.d.ts
import type { And } from "./utils.d.ts";

export type Alias = And<[true, true]>;
```

```ds:main.ds
import type { Alias } from "./branding.d.ts";

declare let value: Alias;
value satisfies true;
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true } }
```

### tuple comptime arguments stay grouped for imported tuple types

> Tuple type arguments stay grouped across imported aliases.

```ds:utils.ds
export type Wrap<T> = T;
export type Alias = Wrap<(number, string)>;
```

```ds:main.ds
import type { Alias } from "./utils.ds";

declare let value: Alias;
value satisfies (number, string);
```

### tuple comptime arguments stay grouped with nested aliases

> Tuple comptime arguments remain grouped through nested alias wrappers.

```ds
type And<Types extends boolean[]> = Types[number] extends true ? true : false;
type Wrap<T> = T;
type Alias = Wrap<And<[true, true]>>;

declare let value: Alias;
value satisfies true;
```

### tuple comptime arguments stay grouped through alias imports

> Type alias references preserve tuple comptime arguments across declaration modules.

```ts:utils.d.ts
export type And<Types extends boolean[]> = Types[number] extends true ? true : false;

export type MutuallyExtends<Left, Right> = And<
  [Left extends Right ? true : false, Right extends Left ? true : false]
>;
```

```ts:branding.d.ts
import type { MutuallyExtends } from "./utils.d.ts";

export type StrictEqualUsingBranding<Left, Right> = MutuallyExtends<Left, Right>;
```

```ds:main.ds
import type { StrictEqualUsingBranding } from "./branding.d.ts";

type Result = StrictEqualUsingBranding<number, number>;

declare let value: Result;
value satisfies true;
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true } }
```

## newtypes

### explicit type arguments on newtypes

> Newtypes accept explicit type arguments.

```ds
newtype Box<T> = { value: T }

declare function makeBox(): Box<number>;

let value: Box<number> = makeBox();
value satisfies Box<number>;
```

### type argument mismatch on newtypes

> Type arguments must satisfy declared bounds.

```ds
newtype Box<T: number> = { value: T }

declare function makeBox(): Box<number>;

let value: Box<string> = makeBox();
```

- type string is not assignable to type number

### comptime value arguments on newtypes

> Comptime value arguments are checked against declared types.

```ds
newtype Buffer<T, comptime N: number> = { value: T }

declare function makeBuffer(): Buffer<string, 4>;

let buffer: Buffer<string, 4> = makeBuffer();
buffer satisfies Buffer<string, 4>;
```

### comptime value arguments on newtypes require static expressions

> Newtype comptime value arguments must be static expressions.

```ds
newtype Buffer<T, comptime N: number> = { value: T }

declare function makeBuffer(): Buffer<string, 4>;

let buffer: Buffer<string, comptime 4> = makeBuffer();
```

- contains: static expression

### comptime value argument mismatch on newtypes

> Comptime value arguments must satisfy declared types.

```ds
newtype Buffer<T, comptime N: number> = { value: T }

declare function makeBuffer(): Buffer<string, 4>;

let buffer: Buffer<string, true> = makeBuffer();
```

- type true is not assignable to type number

### default type parameters on newtypes

> Type parameters fall back to defaults when omitted.

```ds
newtype Box<T = number> = { value: T }

declare function makeBox(): Box;

let value: Box = makeBox();
value satisfies Box<number>;
```

### default comptime value arguments on newtypes

> Comptime value arguments fall back to defaults when omitted.

```ds
newtype Buffer<T, comptime N: number = 4> = { value: T }

declare function makeBuffer(): Buffer<string>;

let buffer: Buffer<string> = makeBuffer();
buffer satisfies Buffer<string, 4>;
```

## interfaces

### explicit type arguments on interfaces

> Interface type references accept explicit type arguments.

```ds
interface Box<T> { value: T }

declare function makeBox(): Box<number>;

let value: Box<number> = makeBox();
value satisfies Box<number>;
```

### type argument mismatch on interfaces

> Type arguments must satisfy declared bounds.

```ds
interface Box<T: number> { value: T }

declare function makeBox(): Box<number>;

let value: Box<string> = makeBox();
```

- type string is not assignable to type number

### comptime value arguments on interfaces

> Comptime value arguments are checked against declared types.

```ds
interface Buffer<T, comptime N: number> { value: T }

declare function makeBuffer(): Buffer<string, 4>;

let buffer: Buffer<string, 4> = makeBuffer();
buffer satisfies Buffer<string, 4>;
```

### comptime value argument mismatch on interfaces

> Comptime value arguments must satisfy declared types.

```ds
interface Buffer<T, comptime N: number> { value: T }

declare function makeBuffer(): Buffer<string, 4>;

let buffer: Buffer<string, true> = makeBuffer();
```

- type true is not assignable to type number

### default type parameters on interfaces

> Type parameters fall back to defaults when omitted.

```ds
interface Box<T = number> { value: T }

declare function makeBox(): Box;

let value: Box = makeBox();
value satisfies Box<number>;
```

### default comptime value arguments on interfaces

> Comptime value arguments fall back to defaults when omitted.

```ds
interface Buffer<T, comptime N: number = 4> { value: T }

declare function makeBuffer(): Buffer<string>;

let buffer: Buffer<string> = makeBuffer();
buffer satisfies Buffer<string, 4>;
```

## structs

### explicit type arguments on structs

> Struct type references accept explicit type arguments.

```ds
struct Box<T> { value: T }

declare function makeBox(): Box<number>;

let value: Box<number> = makeBox();
value satisfies Box<number>;
```

### type argument mismatch on structs

> Type arguments must satisfy declared bounds.

```ds
struct Box<T: number> { value: T }

declare function makeBox(): Box<number>;

let value: Box<string> = makeBox();
```

- type string is not assignable to type number

### comptime value arguments on structs

> Comptime value arguments are checked against declared types.

```ds
struct Buffer<T, comptime N: number> { value: T }

declare function makeBuffer(): Buffer<string, 4>;

let buffer: Buffer<string, 4> = makeBuffer();
buffer satisfies Buffer<string, 4>;
```

### comptime value argument mismatch on structs

> Comptime value arguments must satisfy declared types.

```ds
struct Buffer<T, comptime N: number> { value: T }

declare function makeBuffer(): Buffer<string, 4>;

let buffer: Buffer<string, true> = makeBuffer();
```

- type true is not assignable to type number

### default type parameters on structs

> Type parameters fall back to defaults when omitted.

```ds
struct Box<T = number> { value: T }

declare function makeBox(): Box;

let value: Box = makeBox();
value satisfies Box<number>;
```

### default comptime value arguments on structs

> Comptime value arguments fall back to defaults when omitted.

```ds
struct Buffer<T, comptime N: number = 4> { value: T }

declare function makeBuffer(): Buffer<string>;

let buffer: Buffer<string> = makeBuffer();
buffer satisfies Buffer<string, 4>;
```

## classes

### explicit type arguments on classes

> Class type references accept explicit type arguments.

```ds
class Box<T> {
    value?: T
}

declare function makeBox(): Box<number>;

let value: Box<number> = makeBox();
value satisfies Box<number>;
```

### type argument mismatch on classes

> Type arguments must satisfy declared bounds.

```ds
class Box<T: number> {
    value?: T
}

declare function makeBox(): Box<number>;

let value: Box<string> = makeBox();
```

- type string is not assignable to type number

### comptime value arguments on classes

> Comptime value arguments are checked against declared types.

```ds
class Buffer<T, comptime N: number> {
    value?: T
}

declare function makeBuffer(): Buffer<string, 4>;

let buffer: Buffer<string, 4> = makeBuffer();
buffer satisfies Buffer<string, 4>;
```

### comptime value argument mismatch on classes

> Comptime value arguments must satisfy declared types.

```ds
class Buffer<T, comptime N: number> {
    value?: T
}

declare function makeBuffer(): Buffer<string, 4>;

let buffer: Buffer<string, true> = makeBuffer();
```

- type true is not assignable to type number

### default type parameters on classes

> Type parameters fall back to defaults when omitted.

```ds
class Box<T = number> {
    value?: T
}

declare function makeBox(): Box;

let value: Box = makeBox();
value satisfies Box<number>;
```

### default comptime value arguments on classes

> Comptime value arguments fall back to defaults when omitted.

```ds
class Buffer<T, comptime N: number = 4> {
    value?: T
}

declare function makeBuffer(): Buffer<string>;

let buffer: Buffer<string> = makeBuffer();
buffer satisfies Buffer<string, 4>;
```

### type arguments affect assignability

> Class type references with different type arguments are not assignable.

```ds
class Box<T> {
    value?: T
}

declare let numberBox: Box<number>;

let stringBox: Box<string> = numberBox;
```

- type Buffer<string, 4> is not assignable to type Buffer<string, 8>

### comptime value arguments affect assignability

> Class references with different value arguments are not assignable.

```ds
class Buffer<T, comptime N: number> {
    value?: T
}

declare let buffer4: Buffer<string, 4>;

let buffer8: Buffer<string, 8> = buffer4;
```

- type Buffer<string, 4> is not assignable to type Buffer<string, 8>

## extensions

### extension type parameters

> Generic parameters on extensions flow into member signatures.

```ds
struct Box<T> { value: T }

extension<T> of Box<T> {
    get(): T {
        return this.value;
    }
}

declare function makeBox(): Box<number>;

const boxed = makeBox();
boxed.get() satisfies number;
```

### extension comptime value parameters

> Comptime value parameters on extensions are validated.

```ds
struct Buffer<T, comptime N: number> { value: T }

extension<T, comptime N: number> of Buffer<T, N> {
    get(): T {
        return this.value;
    }
}

declare function makeBuffer(): Buffer<string, 4>;

const buffer = makeBuffer();
buffer.get() satisfies string;
```

### extension generic parameters map by target argument order

> Extension parameters follow the target type argument order.

```ds
struct Pair<A, B> {
    left: A;
    right: B
}

extension<Left, Right> of Pair<Right, Left> {
    swap(): Pair<Left, Right> {
        return Pair<Left, Right> {
            left: this.right,
            right: this.left
        };
    }
}

declare function makePair(): Pair<number, string>;

const pair = makePair();
pair.swap() satisfies Pair<string, number>;
```

### extension comptime value parameters use defaults

> Extensions inherit default comptime arguments from target type references.

```ds
struct Buffer<T, comptime N: number = 4> {
    value: T
}

extension<T, comptime N: number> of Buffer<T, N> {
    get(): T {
        return this.value;
    }
}

declare function makeBuffer(): Buffer<string>;

const buffer = makeBuffer();
buffer.get() satisfies string;
```

### extension generic parameter mismatch rejects incompatible calls

> Extension methods still enforce substituted generic parameter contracts.

```ds
struct Buffer<T, comptime N: number> {
    value: T
}

extension<T, comptime N: number> of Buffer<T, N> {
    requireSize(value: [T; N]): [T; N] {
        return value;
    }
}

declare function makeBuffer(): Buffer<uint8, 4>;

const buffer = makeBuffer();
buffer.requireSize([1, 2, 3, 4]);
buffer.requireSize([1, 2]);
```

- contains: not assignable

### extension defaults keep concrete member types

> Defaulted comptime arguments are visible inside extension methods.

```ds
struct Registry<T, comptime N: number = 2> {
    value: T
}

extension<T, comptime N: number> of Registry<T, N> {
    pair(): (T, T) {
        (this.value, this.value)
    }
}

declare function makeRegistry(): Registry<string>;

const registry = makeRegistry();
registry.pair() satisfies (string, string);
```
