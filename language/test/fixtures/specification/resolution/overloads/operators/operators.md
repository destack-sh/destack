# Operator Overloading

Tests for operator overloading via interface implementations.

## Addition

### extension implements Add

> The `+` operator dispatches to the `add` method on the receiver.

```ds
struct Vector2 { x: number; y: number }

extension of Vector2 implements Add<Vector2> {
    add(other: Vector2): Vector2 {
        return Vector2 { x: 0, y: 0 }
    }
}

declare function getVector(): Vector2;

const left = getVector();
const right = getVector();
const sum = left + right;
sum satisfies Vector2;
```

### extension without interface does not overload

> The `+` operator requires an explicit `implements Add` clause.

```ds
struct Vector2 { x: number; y: number }

extension of Vector2 {
    add(other: Vector2): Vector2 {
        return Vector2 { x: 0, y: 0 }
    }
}

declare function getVector(): Vector2;

const left = getVector();
const right = getVector();
left + right;
```

- contains: no matching overload

## Builtin overloading

### array concatenation uses builtin overload

> Arrays support `+` concatenation when builtin overloads are available.

```ds
declare const left: int32[];
declare const right: int32[];

const combined = left + right;
combined satisfies int32[];
```

### set union uses builtin overload

> Sets support `|` union when builtin overloads are available.

```ds libs=es2015
declare const left: Set<int32>;
declare const right: Set<int32>;

const combined = left | right;
combined satisfies Set<int32>;
```

### map merge uses builtin overload

> Maps support `|` merge when builtin overloads are available.

```ds libs=es2015
declare const left: Map<string, int32>;
declare const right: Map<string, int32>;

const combined = left | right;
combined satisfies Map<string, int32>;
```

## Module ordering

### receiver ordering through renamed re-exports keeps first applicable overload

> Renamed re-export paths should preserve receiver overload ordering.

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

```ds:index.ds
export { Counter as PublicCounter } from "./counter";
```

```ds:main.ds
import { PublicCounter } from "./index";
import { CounterNumberAdd, CounterIntAdd } from "./extensions";

declare let counter: PublicCounter;

const selected = counter + 1;
selected satisfies "number";
```

### receiver ordering through renamed re-exports rejects later overload expectations

> Renamed re-export paths should not select later receiver overloads when earlier ones apply.

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

```ds:index.ds
export { Counter as PublicCounter } from "./counter";
```

```ds:main.ds
import { PublicCounter } from "./index";
import { CounterNumberAdd, CounterIntAdd } from "./extensions";

declare let counter: PublicCounter;

const selected = counter + 1;
selected satisfies "int32";
```

- expected "int32", found "number" (not assignable)