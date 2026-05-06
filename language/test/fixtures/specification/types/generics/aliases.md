# Generic Aliases

Type aliases can bind type parameters and static value parameters.

## arguments

### aliases accept explicit type arguments

Type aliases accept explicit type arguments.

```ds
type Box<T> = { value: T }

declare function makeBox(): Box<number>;

let value: Box<number> = makeBox();
value satisfies Box<number>;
```

### aliases reject type argument mismatches

Type arguments must satisfy declared bounds.

```ds
type Box<T: number> = { value: T }

declare function makeBox(): Box<number>;

let value: Box<string> = makeBox();
```

- contains: not assignable

### aliases accept static value arguments

Static value arguments are checked against declared types.

```ds
type Buffer<T, comptime N: number> = { value: T }

declare function makeBuffer(): Buffer<string, 4>;

let buffer: Buffer<string, 4> = makeBuffer();
buffer satisfies Buffer<string, 4>;
```

### aliases require static value arguments

Static value arguments must be static expressions.

```ds
type Buffer<T, comptime N: number> = { value: T }

declare function makeBuffer(): Buffer<string, 4>;

let buffer: Buffer<string, comptime 4> = makeBuffer();
```

- contains: static expression

### aliases reject static value argument mismatches

Static value arguments must satisfy declared types.

```ds
type Buffer<T, comptime N: number> = { value: T }

declare function makeBuffer(): Buffer<string, 4>;

let buffer: Buffer<string, true> = makeBuffer();
```

- contains: not assignable

## defaults

### aliases accept default type parameters

Type parameters fall back to defaults when omitted.

```ds
type Box<T = number> = { value: T }

declare function makeBox(): Box;

let value: Box = makeBox();
value satisfies Box<number>;
```

### aliases accept default static values

Static value arguments fall back to defaults when omitted.

```ds
type Buffer<T, comptime N: number = 4> = { value: T }

declare function makeBuffer(): Buffer<string>;

let buffer: Buffer<string> = makeBuffer();
buffer satisfies Buffer<string, 4>;
```

### alias static defaults must be static

Static value defaults must be static expressions.

```ds
type Buffer<T, comptime N: number = comptime 4> = { value: T }

declare function makeBuffer(): Buffer<string>;

let buffer: Buffer<string> = makeBuffer();
```

- contains: static expression

## tuple arguments

### tuple arguments stay grouped for direct aliases

Tuple static arguments stay grouped when passed through aliases.

```ds
type And<Types extends (boolean, boolean)> =
    Types[0] extends true ? (Types[1] extends true ? true : false) : false;
type MutuallyExtends<Left, Right> = And<
  (Left extends Right ? true : false, Right extends Left ? true : false)
>;

type Result = MutuallyExtends<number, number>;

declare let value: Result;
value satisfies true;
```

### tuple type arguments stay grouped

Tuple type arguments remain grouped for direct aliases.

```ds
type Wrap<T> = T;
type Alias = Wrap<(number, string)>;

declare let value: Alias;
value satisfies (number, string);
```

### imported tuple arguments stay grouped

Tuple static arguments stay grouped across imported aliases.

```ds:utils.ds
export type And<Types extends (boolean, boolean)> =
    Types[0] extends true ? (Types[1] extends true ? true : false) : false;
```

```ds:branding.ds
import type { And } from "./utils.ds";

export type Alias = And<(true, true)>;
```

```ds:main.ds
import type { Alias } from "./branding.ds";

declare let value: Alias;
value satisfies true;
```

### imported tuple types stay grouped

Tuple type arguments stay grouped across imported aliases.

```ds:utils.ds
export type Wrap<T> = T;
export type Alias = Wrap<(number, string)>;
```

```ds:main.ds
import type { Alias } from "./utils.ds";

declare let value: Alias;
value satisfies (number, string);
```

### nested tuple arguments stay grouped

Tuple static arguments remain grouped through nested alias wrappers.

```ds
type And<Types extends (boolean, boolean)> =
    Types[0] extends true ? (Types[1] extends true ? true : false) : false;
type Wrap<T> = T;
type Alias = Wrap<And<(true, true)>>;

declare let value: Alias;
value satisfies true;
```

### imported alias chains preserve tuple arguments

Type alias references preserve tuple static arguments across imports.

```ds:utils.ds
export type And<Types extends (boolean, boolean)> =
    Types[0] extends true ? (Types[1] extends true ? true : false) : false;

export type MutuallyExtends<Left, Right> = And<
  (Left extends Right ? true : false, Right extends Left ? true : false)
>;
```

```ds:branding.ds
import type { MutuallyExtends } from "./utils.ds";

export type StrictEqualUsingBranding<Left, Right> = MutuallyExtends<Left, Right>;
```

```ds:main.ds
import type { StrictEqualUsingBranding } from "./branding.ds";

type Result = StrictEqualUsingBranding<number, number>;

declare let value: Result;
value satisfies true;
```
