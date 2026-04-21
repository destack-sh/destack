# Receiver Based Overload Selection

Receiver based dispatch should first select the receiver family, then apply declaration order.
Overload order should be honored even when overloads come from multiple extensions.

## Operator families

### receiver overload order applies across extensions

> Extensions on the same receiver should contribute overloads in declaration order.

```ds
struct Counter {}

extension of Counter implements Add<number> {
    add(other: number): "number" { return "number" }
}

extension of Counter implements Add<int32> {
    add(other: int32): "int32" { return "int32" }
}

declare let counter: Counter;

const selected = counter + 1;
selected satisfies "number";
```

### receiver overload order rejects later overload results

> Later receiver overloads should not win when earlier overloads apply.

```ds
struct Counter {}

extension of Counter implements Add<number> {
    add(other: number): "number" { return "number" }
}

extension of Counter implements Add<int32> {
    add(other: int32): "int32" { return "int32" }
}

declare let counter: Counter;

const selected = counter + 1;
selected satisfies "int32";
```

- contains: not assignable

### receiver overloads can be ordered for specificity

> Specific receiver overloads should be declared first when they should win.

```ds
struct Counter {}

extension of Counter implements Add<int32> {
    add(other: int32): "int32" { return "int32" }
}

extension of Counter implements Add<number> {
    add(other: number): "number" { return "number" }
}

declare let counter: Counter;

const selected = counter + 1;
selected satisfies "int32";
```

### receiver overload ordering rejects later broad overloads

> Later broad overloads should not win when specific overloads are first.

```ds
struct Counter {}

extension of Counter implements Add<int32> {
    add(other: int32): "int32" { return "int32" }
}

extension of Counter implements Add<number> {
    add(other: number): "number" { return "number" }
}

declare let counter: Counter;

const selected = counter + 1;
selected satisfies "number";
```

- contains: not assignable

### receiver overload order is preserved through exported extension modules

> Overload declaration order should remain stable when extensions are imported from other modules.

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

const selected = counter + 1;
selected satisfies "number";
```

### receiver overload order through modules does not select later overloads

> Imported extension overload sets should not promote later declarations.

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

const selected = counter + 1;
selected satisfies "int32";
```

- contains: not assignable

### receiver overload order survives export-star plus rename chains

> Export-star and rename chains should preserve receiver overload ordering.

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

```ds:barrel.ds
export * from "./counter";
```

```ds:index.ds
export * from "./barrel";
export { Counter as PublicCounter } from "./counter";
```

```ds:main.ds
import { PublicCounter } from "./index";
import {
    CounterNumberAdd as PublicCounterNumberAdd,
    CounterIntAdd as PublicCounterIntAdd
} from "./extensions";

declare let counter: PublicCounter;

const selected = counter + 1;
selected satisfies "number";
```
