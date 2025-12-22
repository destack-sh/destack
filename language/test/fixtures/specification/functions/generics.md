# Generics

Tests for static parameter instancing and inference.

## functions

### explicit static type arguments

> Explicit static type arguments specialize function references.

```ds
function identity<T>(value: T): T {
    return value
}
const as_number = identity<number>;
as_number satisfies (value: number) => number;
```

### inferred static type arguments from call

> Static type parameters are inferred from call arguments.

```ds
function identity<T>(value: T): T {
    return value
}
const one = identity(1);
one satisfies 1;
```

### explicit static type argument mismatch

> Call arguments must satisfy specialized parameter types.

```ds
function identity<T>(value: T): T {
    return value
}
identity<number>("hi");
```

- contains: type "hi" is not assignable to type number

### static value arguments

> Static value arguments are checked against declared types.

```ds
function choose<Flag: boolean>(value: number): number {
    return value
}
choose<true>(1);
choose<1>(1);
```

- contains: type 1 is not assignable to type boolean

### default static value arguments

> Static value arguments fall back to defaults when omitted.

```ds
function choose<Flag: boolean = true>(value: number): number {
    return value
}
choose(1);
```

### default static type parameters

> Static type parameters fall back to defaults when omitted.

```ds
declare function make<T = number>(): T;

const value = make();
value satisfies number;
```

### member static arguments on member expressions

> Member static arguments are applied before call typing.

```ds
interface Container<T> {
    map<U>(value: T): U
}

declare function getContainer(): Container<number>;

const mapper = getContainer().map<string>;
mapper("hi");
```

- contains: type "hi" is not assignable to type number

### _conflicting static arguments on member calls

> nocheckin #Broken: parser does not attach postfix static arguments to non path expressions, so this syntax never produces both member and call static arguments, add postfix instantiation parsing for any expression when `<...>` precedes `(` to make the conflict representable

> Static arguments cannot appear on both a member and its call.

```ds
interface Container<T> {
    map<U>(value: T): U
}

declare function getContainer(): Container<number>;

(getContainer().map<string>)<number>(1);
```

- contains: static arguments specified on both member and call
