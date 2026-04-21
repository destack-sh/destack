# Dispatch Overloads

## receiver and rhs legality

### receiver overload order wins when multiple overloads apply

For receiver calls, overload resolution should commit to the first compatible receiver signature by declaration order.

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

### later receiver overloads do not override earlier ones

A later receiver overload must not replace an already applicable earlier receiver candidate.

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
value satisfies "int32";
```

- contains: not assignable

### rhs only implementations are not selected

When only a receiver form exists, rhs-only implementations should not be considered callable.

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

### imported extension modules preserve receiver family ordering

Extension overloads imported from modules should keep stable receiver-family ordering during dispatch.

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
