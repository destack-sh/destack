# Receiver Based Overload Selection

Receiver based dispatch should first select the receiver family, then apply declaration order.
Overload order should be honored even when overloads come from multiple extensions.

## Operator families

### receiver overload order applies across extensions

> Extensions on the same receiver should contribute overloads in declaration order.

```ds
struct Counter {}

extension for Counter implements Add<number> {
    add(other: number): "number" { return "number" }
}

extension for Counter implements Add<int32> {
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

extension for Counter implements Add<number> {
    add(other: number): "number" { return "number" }
}

extension for Counter implements Add<int32> {
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

extension for Counter implements Add<int32> {
    add(other: int32): "int32" { return "int32" }
}

extension for Counter implements Add<number> {
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

extension for Counter implements Add<int32> {
    add(other: int32): "int32" { return "int32" }
}

extension for Counter implements Add<number> {
    add(other: number): "number" { return "number" }
}

declare let counter: Counter;

const selected = counter + 1;
selected satisfies "number";
```

- contains: not assignable
