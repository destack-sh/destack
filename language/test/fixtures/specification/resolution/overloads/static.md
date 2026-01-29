# Overloads With Static Parameters

Static value parameters should not change overload ranking rules.
They should only affect whether an overload is applicable.

## Applicability

### comptime overloads are skipped when static arguments cannot be inferred

> A comptime overload should not block later overloads when its static arguments cannot be inferred.

```ds
function make<comptime N: number>(value: uint8[N]): "static" {
    return "static";
}

function make(value: uint8[]): "dynamic" {
    return "dynamic";
}

let data: uint8[] = [1, 2, 3, 4];

const selected = make(data);
selected satisfies "dynamic";
```

## Known static arguments

### comptime overloads apply when static arguments are known

> A comptime overload should apply when static arguments are known at the call site.

```ds
function make<comptime N: number>(value: uint8[N]): "static" {
    return "static";
}

function make(value: uint8[]): "dynamic" {
    return "dynamic";
}

const data: uint8[4] = [1, 2, 3, 4];

const selected = make(data);
selected satisfies "static";
```

### declaration order still wins when static arguments are known

> Static parameters should not outrank earlier overloads when both are applicable.

```ds
function make(value: uint8[]): "dynamic" {
    return "dynamic";
}

function make<comptime N: number>(value: uint8[N]): "static" {
    return "static";
}

const data: uint8[4] = [1, 2, 3, 4];

const selected = make(data);
selected satisfies "dynamic";
```

### declaration order does not select later static overloads

> Later static overloads should not win when earlier overloads apply.

```ds
function make(value: uint8[]): "dynamic" {
    return "dynamic";
}

function make<comptime N: number>(value: uint8[N]): "static" {
    return "static";
}

const data: uint8[4] = [1, 2, 3, 4];

const selected = make(data);
selected satisfies "static";
```

- contains: not assignable
