# Calls

Function calls check receivers, argument tuples, and overload sets directly.

## call

### call checks this arguments

> `call` enforces the target `this` type.

```ds
function add(this: { base: number }, value: number): number {
    return this.base + value;
}

add.call({ base: "no" }, 1);
```

- contains: not assignable

### call selects overloads in declaration order

> `call` uses the same overload ordering as a direct call.

```ds
function choose(value: "ready"): "ready" {
    return value;
}

function choose(value: string): string {
    return value;
}

const result = choose.call(undefined, "ready");
result satisfies "ready";
```

## apply

### apply checks argument tuples

> `apply` validates tuple arguments against the function signature.

```ds
function add(this: { base: number }, value: number): number {
    return this.base + value;
}

add.apply({ base: 1 }, ["no"]);
```

- contains: not assignable

### apply selects overloads in declaration order

> `apply` keeps overload declaration order.

```ds
function choose(value: "ready"): "ready" {
    return value;
}

function choose(value: string): string {
    return value;
}

const result = choose.apply(undefined, ["ready"]);
result satisfies "ready";
```

## bind

### bind preserves parameter types

> `bind` returns a function with the remaining parameter types.

```ds
function add(this: { base: number }, value: number): number {
    return this.base + value;
}

const bound = add.bind({ base: 1 });
const result = bound(2);
result satisfies number;
```

### bind checks later call sites

> Bound functions preserve parameter type checking.

```ds
function add(this: { base: number }, value: number): number {
    return this.base + value;
}

const bound = add.bind({ base: 1 });
bound("no");
```

- contains: not assignable
