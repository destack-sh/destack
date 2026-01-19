# Static Arguments for Type Aliases and Newtypes

Tests for static parameters on type aliases and newtypes.

## type aliases

### explicit static type arguments on type aliases

> Type aliases accept explicit static type arguments.

```ds
type Box<T> = { value: T }

declare function makeBox(): Box<number>;

let value: Box<number> = makeBox();
value satisfies Box<number>;
```

### static type argument mismatch on type aliases

> Static type arguments must satisfy declared bounds.

```ds
type Box<T: number> = { value: T }

declare function makeBox(): Box<number>;

let value: Box<string> = makeBox();
```

- contains: type string is not assignable to type number

### static value arguments on type aliases

> Static value arguments are checked against declared types.

```ds
type Buffer<T, N: number> = { value: T }

declare function makeBuffer(): Buffer<string, 4>;

let buffer: Buffer<string, 4> = makeBuffer();
buffer satisfies Buffer<string, 4>;
```

### static value arguments require static expressions

> Static value arguments must be static expressions.

```ds
type Buffer<T, N: number> = { value: T }

declare function makeBuffer(): Buffer<string, 4>;

let buffer: Buffer<string, comptime 4> = makeBuffer();
```

- contains: static argument must be a static expression

### static value argument mismatch on type aliases

> Static value arguments must satisfy declared types.

```ds
type Buffer<T, N: number> = { value: T }

declare function makeBuffer(): Buffer<string, 4>;

let buffer: Buffer<string, true> = makeBuffer();
```

- contains: type true is not assignable to type number

### default static type parameters on type aliases

> Static type parameters fall back to defaults when omitted.

```ds
type Box<T = number> = { value: T }

declare function makeBox(): Box;

let value: Box = makeBox();
value satisfies Box<number>;
```

### default static value arguments on type aliases

> Static value arguments fall back to defaults when omitted.

```ds
type Buffer<T, N: number = 4> = { value: T }

declare function makeBuffer(): Buffer<string>;

let buffer: Buffer<string> = makeBuffer();
buffer satisfies Buffer<string, 4>;
```

### default static value arguments require static expressions

> Static value defaults must be static expressions.

```ds
type Buffer<T, N: number = comptime 4> = { value: T }

declare function makeBuffer(): Buffer<string>;

let buffer: Buffer<string> = makeBuffer();
```

- contains: static argument must be a static expression

### tuple static arguments stay grouped for direct aliases

> Tuple static arguments stay grouped when passed through aliases.

```ds
type And<Types extends boolean[]> = Types[number] extends true ? true : false;
type MutuallyExtends<Left, Right> = And<
  [Left extends Right ? true : false, Right extends Left ? true : false]
>;

type Result = MutuallyExtends<number, number>;

declare let value: Result;
value satisfies true;
```

### tuple static arguments stay grouped for tuple types

> Tuple type arguments remain grouped for direct aliases.

```ds
type Wrap<T> = T;
type Alias = Wrap<(number, string)>;

declare let value: Alias;
value satisfies (number, string);
```

### tuple static arguments stay grouped for imported aliases

> Tuple static arguments stay grouped across imported aliases.

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

```ds:dsconfig.json
{ "compilerOptions": { "allowTs": true, "checkTs": true } }
```

### tuple static arguments stay grouped for imported tuple types

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

### tuple static arguments stay grouped with nested aliases

> Tuple static arguments remain grouped through nested alias wrappers.

```ds
type And<Types extends boolean[]> = Types[number] extends true ? true : false;
type Wrap<T> = T;
type Alias = Wrap<And<[true, true]>>;

declare let value: Alias;
value satisfies true;
```

### tuple static arguments stay grouped through alias imports

> Type alias references preserve tuple static arguments across declaration modules.

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

```ds:dsconfig.json
{ "compilerOptions": { "allowTs": true, "checkTs": true } }
```

## newtypes

### explicit static type arguments on newtypes

> Newtypes accept explicit static type arguments.

```ds
newtype Box<T> = { value: T }

declare function makeBox(): Box<number>;

let value: Box<number> = makeBox();
value satisfies Box<number>;
```

### static type argument mismatch on newtypes

> Static type arguments must satisfy declared bounds.

```ds
newtype Box<T: number> = { value: T }

declare function makeBox(): Box<number>;

let value: Box<string> = makeBox();
```

- contains: type string is not assignable to type number

### static value arguments on newtypes

> Static value arguments are checked against declared types.

```ds
newtype Buffer<T, N: number> = { value: T }

declare function makeBuffer(): Buffer<string, 4>;

let buffer: Buffer<string, 4> = makeBuffer();
buffer satisfies Buffer<string, 4>;
```

### static value arguments on newtypes require static expressions

> Newtype static value arguments must be static expressions.

```ds
newtype Buffer<T, N: number> = { value: T }

declare function makeBuffer(): Buffer<string, 4>;

let buffer: Buffer<string, comptime 4> = makeBuffer();
```

- contains: static argument must be a static expression

### static value argument mismatch on newtypes

> Static value arguments must satisfy declared types.

```ds
newtype Buffer<T, N: number> = { value: T }

declare function makeBuffer(): Buffer<string, 4>;

let buffer: Buffer<string, true> = makeBuffer();
```

- contains: type true is not assignable to type number

### default static type parameters on newtypes

> Static type parameters fall back to defaults when omitted.

```ds
newtype Box<T = number> = { value: T }

declare function makeBox(): Box;

let value: Box = makeBox();
value satisfies Box<number>;
```

### default static value arguments on newtypes

> Static value arguments fall back to defaults when omitted.

```ds
newtype Buffer<T, N: number = 4> = { value: T }

declare function makeBuffer(): Buffer<string>;

let buffer: Buffer<string> = makeBuffer();
buffer satisfies Buffer<string, 4>;
```
