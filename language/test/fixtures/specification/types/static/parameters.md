# Static Parameters

Static value parameters are generic parameters that carry compile-time values.
They must be marked with `comptime`.

## parameters

### static value parameters require comptime modifiers

Value parameters must be marked with `comptime`.

```ds
type Buffer<N: uint> = [uint8; N];

declare let value: Buffer<4>;
```

- contains: comptime

### static value parameters apply to sized arrays

Value parameters can drive array sizes in type expressions.

```ds
type Buffer<comptime N: uint> = [uint8; N];

declare let value: Buffer<4>;

value satisfies [uint8; 4];
```

### sized arrays are assignable to dynamic arrays

Sized arrays are assignable to dynamic arrays with compatible element types.

```ds
type Buffer<comptime N: uint> = [uint8; N];

declare let value: Buffer<4>;

value satisfies uint8[];
```

### static value parameters pass through type aliases

Static value parameters can be passed through type aliases.

```ds
type Buffer<comptime N: uint> = [uint8; N];
type Outer<comptime M: uint> = Buffer<M>;

declare let value: Outer<4>;

value satisfies [uint8; 4];
```

### static value defaults can reference earlier parameters

Default value parameters may reference earlier parameters.

```ds
type Buffer<comptime N: uint, comptime M: uint = N> = [uint8; M];

declare let value: Buffer<4>;

value satisfies [uint8; 4];
```

### static value parameters accept string literals

Value parameters accept string literal comptime arguments.

```ds
type Tagged<comptime Tag: string> = { tag: Tag };

declare let value: Tagged<"alpha">;
value satisfies Tagged<"alpha">;
```

### static value parameters accept boolean literals

Value parameters accept boolean literal comptime arguments.

```ds
type Flagged<comptime Enabled: boolean> = { enabled: Enabled };

declare let value: Flagged<true>;
value satisfies Flagged<true>;
```

### static value parameters accept bigint literals

Value parameters accept bigint literal comptime arguments.

```ds
type BigLimit<comptime N: bigint> = { limit: N };

declare let value: BigLimit<42n>;
value satisfies BigLimit<42n>;
```

### static value parameters accept arithmetic expressions

Value parameters accept static arithmetic expressions.

```ds
type Buffer<comptime N: uint> = [uint8; N];

declare let value: Buffer<2 + 2>;
value satisfies [uint8; 4];
```

### static value parameters accept string unions

Value parameters accept literal unions of strings.

```ds
type Tagged<comptime Tag: "fast" | "slow"> = { tag: Tag };

declare let value: Tagged<"fast">;
value satisfies Tagged<"fast">;
```

### static value parameters reject non members from unions

Literal unions reject values outside the union.

```ds
type Tagged<comptime Tag: "fast" | "slow"> = { tag: Tag };

declare let value: Tagged<"medium">;
```

- contains: not assignable

### static value parameters reject mismatched string arguments

String value parameters reject non string arguments.

```ds
type Tagged<comptime Tag: string> = { tag: Tag };

declare let value: Tagged<1>;
```

- contains: not assignable

### static value parameters reject mismatched boolean arguments

Boolean value parameters reject non boolean arguments.

```ds
type Flagged<comptime Enabled: boolean> = { enabled: Enabled };

declare let value: Flagged<1>;
```

- contains: not assignable

### static value parameters reject mismatched bigint arguments

Bigint value parameters reject non bigint arguments.

```ds
type BigLimit<comptime N: bigint> = { limit: N };

declare let value: BigLimit<1>;
```

- contains: not assignable

### static value parameters reject non static expressions

Value parameters reject non static expressions.

```ds
let size = 4;

type Buffer<comptime N: uint> = [uint8; N];

declare let value: Buffer<size>;
```

- contains: static expression

### static value parameters accept enum members

Enum members are accepted value arguments for matching enum types.

```ds
enum Mode {
    Fast = "fast"
    Slow = "slow"
}

type Run<comptime M: Mode> = { mode: M };

declare let value: Run<Mode.Fast>;
value satisfies Run<Mode.Fast>;
```

### static value parameters accept imported enum members

Imported enum members are accepted value arguments.

```ds:utils.ds
export enum Mode {
    Fast = "fast"
    Slow = "slow"
}
```

```ds:main.ds
import { Mode } from "./utils.ds";

type Run<comptime M: Mode> = { mode: M };

declare let value: Run<Mode.Fast>;
value satisfies Run<Mode.Fast>;
```

### static value parameters reject enum members for numeric types

Enum members do not coerce to their backing types.

```ds
enum Mode {
    Fast = 1
    Slow = 2
}

type Run<comptime N: int32> = { value: N };

declare let value: Run<Mode.Fast>;
```

- contains: not assignable

### static value parameters accept tuple expressions

Tuple static expressions may be used as value arguments.

```ds
type Sized<comptime Size: (int32, int32)> = { size: Size };

declare let value: Sized<(4, 8)>;
value satisfies Sized<(4, 8)>;
```

### static value parameters reject mismatched tuples

Tuple static arguments must match the declared tuple type.

```ds
type Sized<comptime Size: (int32, int32)> = { size: Size };

declare let value: Sized<(4, "no")>;
```

- contains: not assignable

### static value parameters accept array expressions

Array static expressions may be used as value arguments.

```ds
type Listed<comptime Values: int32[]> = { values: Values };

declare let value: Listed<[1, 2, 3]>;
value satisfies Listed<[1, 2, 3]>;
```

### static value parameters reject mismatched arrays

Array comptime arguments must match the declared element type.

```ds
type Listed<comptime Values: int32[]> = { values: Values };

declare let value: Listed<[1, true]>;
```

- contains: not assignable

### static value parameters accept object expressions

Object static expressions may be used as value arguments.

```ds
type Tagged<comptime Tag: { name: string, count: int32 }> = { tag: Tag };

declare let value: Tagged<{ name: "alpha", count: 1 }>;
value satisfies Tagged<{ name: "alpha", count: 1 }>;
```

### static value parameters reject mismatched objects

Object comptime arguments must match the declared shape.

```ds
type Tagged<comptime Tag: { name: string, count: int32 }> = { tag: Tag };

declare let value: Tagged<{ name: "alpha", count: true }>;
```

- contains: not assignable

### static value inference uses literal arguments

Literal arguments can infer value parameters.

```ds
type Buffer<comptime N: uint> = [uint8; N];

declare function make<comptime N: uint>(value: [uint8; N]): Buffer<N>;

let value = make([1, 2, 3, 4]);
value satisfies [uint8; 4];
```

### static value inference uses boolean literals

Boolean literal arguments infer boolean value parameters.

```ds
type Flagged<comptime Enabled: boolean> = { enabled: Enabled };

declare function make<comptime Enabled: boolean>(value: Enabled): Flagged<Enabled>;

let value = make(true);
value satisfies Flagged<true>;
```

### static value inference uses string literals

String literal arguments infer string value parameters.

```ds
type Tagged<comptime Tag: string> = { tag: Tag };

declare function make<comptime Tag: string>(value: Tag): Tagged<Tag>;

let value = make("alpha");
value satisfies Tagged<"alpha">;
```

### static value inference uses number literals

Number literal arguments infer number value parameters.

```ds
type Sized<comptime N: number> = { size: N };

declare function make<comptime N: number>(value: N): Sized<N>;

let value = make(4);
value satisfies Sized<4>;
```

### static value inference uses bigint literals

Bigint literal arguments infer bigint value parameters.

```ds
type Sized<comptime N: bigint> = { size: N };

declare function make<comptime N: bigint>(value: N): Sized<N>;

let value = make(4n);
value satisfies Sized<4n>;
```

### static value inference accepts tuple literals

Tuple literals can infer tuple value parameters.

```ds
type Sized<comptime Size: (int32, int32)> = { size: Size };

declare function make<comptime Size: (int32, int32)>(value: Size): Sized<Size>;

let value = make((4, 8));
value satisfies Sized<(4, 8)>;
```

### static value inference accepts object literals

Object literals can infer object value parameters.

```ds
type Tagged<comptime Tag: { name: string, count: int32 }> = { tag: Tag };

declare function make<comptime Tag: { name: string, count: int32 }>(value: Tag): Tagged<Tag>;

let value = make({ name: "alpha", count: 1 });
value satisfies Tagged<{ name: "alpha", count: 1 }>;
```

### static value inference accepts enum members

Enum member arguments infer enum value parameters.

```ds
enum Mode {
    Fast = "fast"
    Slow = "slow"
}

type Run<comptime M: Mode> = { mode: M };

declare function make<comptime M: Mode>(value: M): Run<M>;

let value = make(Mode.Fast);
value satisfies Run<Mode.Fast>;
```

### static value parameters accept local constants

Comptime arguments may reference local constants with static initializers.

```ds
const SIZE = 4;

type Buffer<comptime N: uint> = [uint8; N];

declare let value: Buffer<SIZE>;
value satisfies [uint8; 4];
```

### static value parameters accept imported constants

Imported constants with static initializers are accepted comptime arguments.

```ds:utils.ds
export const SIZE = 4;
```

```ds:main.ds
import { SIZE } from "./utils.ds";

type Buffer<comptime N: uint> = [uint8; N];

declare let value: Buffer<SIZE>;
value satisfies [uint8; 4];
```

### static value inference rejects non literals

Non literal arguments do not infer value parameters.

```ds
type Buffer<comptime N: uint> = [uint8; N];

declare function make<comptime N: uint>(value: [uint8; N]): Buffer<N>;

let data = [1, 2, 3, 4];
let value = make(data);
```

- contains: argument

### static value parameters reject non static constants

Constants with runtime initializers cannot be used as static arguments.

```ds
function size(): int32 {
    return 4;
}

const SIZE = size();

type Buffer<comptime N: uint> = [uint8; N];

declare let value: Buffer<SIZE>;
```

- contains: static expression

## inference

### value parameter inference preserves literal values

Inferred value parameters keep literal precision.

```ds
type Buffer<comptime N: uint> = [uint8; N];

declare function size<comptime N: uint>(value: Buffer<N>): N;

declare let buf: Buffer<4>;

const n = size(buf);
n satisfies 4;
```

## cross module

### value parameter inference remains precise across modules

Cross-module inference preserves literal value parameters without forcing remote inference.

```ds:api.ds
export type Buffer<comptime N: uint> = [uint8; N];

export declare function size<comptime N: uint>(value: Buffer<N>): N;
```

```ds:main.ds
import type { Buffer } from "./api.ds";
import { size } from "./api.ds";

declare let buf: Buffer<4>;

const n = size(buf);
n satisfies 4;
```
