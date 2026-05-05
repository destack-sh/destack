# Operator Dispatch

Operator overloads use the left operand as the receiver.

## receiver

### binary operators use the left receiver

> Receiver dispatch only searches the left operand.

```ds
struct Left {}
struct Right {}

extension of Right implements Add<Left> {
    add(other: Left): Right { return Right {} }
}

declare let left: Left;
declare let right: Right;

left + right;
```

- contains: no matching overload

### reversed operands need reversed implementations

> Reversed operand order uses the reversed receiver.

```ds
struct Vector2 { x: int32; y: int32 }
struct Scalar { value: int32 }

extension of Vector2 implements Add<Scalar> {
    add(other: Scalar): Vector2 { return this }
}

extension of Scalar implements Add<Vector2, Vector2> {
    add(other: Vector2): Vector2 { return other }
}

declare let vector: Vector2;
declare let scalar: Scalar;

const padded = vector + scalar;
const scaled = scalar + vector;

padded satisfies Vector2;
scaled satisfies Vector2;
```

### operator implementations use declaration order

> The first compatible operator implementation wins.

```ds
struct Counter {}

extension of Counter implements Add<number> {
    add(other: number): "number" { return "number" }
}

extension of Counter implements Add<int32> {
    add(other: int32): "int32" { return "int32" }
}

declare let counter: Counter;

const value = counter + 1;
value satisfies "number";
```

## imports

### imported operator order is stable

> Imports do not reorder operator implementations.

```ds:counter.ds
export struct Counter {}
```

```ds:extensions.ds
import { Counter } from "./counter";

export extension CounterNumberAdd of Counter implements Add<number> {
    add(other: number): "number" { return "number" }
}

export extension CounterIntAdd of Counter implements Add<int32> {
    add(other: int32): "int32" { return "int32" }
}
```

```ds:main.ds
import { Counter } from "./counter";
import { CounterNumberAdd, CounterIntAdd } from "./extensions";

declare let counter: Counter;

const value = counter + 1;
value satisfies "number";
```

### overlapping imported operators are ambiguous

> Distinct imported operator implementations cannot silently order overlapping candidates.

```ds:counter.ds
export struct Counter {}
```

```ds:first.ds
import { Counter } from "./counter";

export extension FirstAdd of Counter implements Add<number> {
    add(other: number): "first" { return "first" }
}
```

```ds:second.ds
import { Counter } from "./counter";

export extension SecondAdd of Counter implements Add<number> {
    add(other: number): "second" { return "second" }
}
```

```ds:main.ds
import { Counter } from "./counter";
import { FirstAdd } from "./first";
import { SecondAdd } from "./second";

declare let counter: Counter;

counter + 1;
```

- contains: ambiguous
