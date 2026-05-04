# Generics

Generic parameter instancing and inference.

## functions

### explicit type arguments

> Explicit type arguments specialize function references.

```ds
function identity<T>(value: T): T {
    return value
}
const as_number = identity<number>;
as_number satisfies (value: number) => number;
```

### inferred type arguments from call

> Type parameters are inferred from call arguments.

```ds
function identity<T>(value: T): T {
    return value
}
const one = identity(1);
one satisfies 1;
```

### explicit type argument mismatch

> Call arguments must satisfy concrete parameter types.

```ds
function identity<T>(value: T): T {
    return value
}
identity<number>("hi");
```

- type "hi" is not assignable to type number

### comptime value arguments

> Comptime value arguments are checked against declared types.

```ds
function choose<comptime Flag: boolean>(value: number): number {
    return value
}
choose<true>(1);
choose<1>(1);
```

- type 1 is not assignable to type boolean

### default comptime value arguments

> Comptime value arguments fall back to defaults when omitted.

```ds
function choose<comptime Flag: boolean = true>(value: number): number {
    return value
}
choose(1);
```

### default comptime value argument mismatch

> Default comptime values must satisfy declared types.

```ds
function broken<comptime Flag: boolean = 1>(value: number): number {
    return value
}
broken(1);
```

- type 1 is not assignable to type boolean

### default type parameters

> Type parameters fall back to defaults when omitted.

```ds
declare function make<T = number>(): T;

const value = make();
value satisfies number;
```

### inherited comptime arguments on member calls

> Member calls apply inherited comptime arguments from the receiver type.

```ds
interface Container<T> {
    map<U>(value: T): U
}

declare function getContainer(): Container<number>;

getContainer().map<string>("hello");
```

- type "hello" is not assignable to type number

### member comptime arguments on member expressions

> Member comptime arguments are applied before call typing.

```ds
interface Container<T> {
    map<U>(value: T): U
}

declare function getContainer(): Container<number>;

const mapper = getContainer().map<string>;
mapper("hi");
```

- type "hi" is not assignable to type number

### instantiation expressions require parentheses before member access

> Instantiation expressions must be parenthesized before member or index access.

```ds
function make<T>(value: T): T {
    return value
}

make<number>.value;
```

- instantiation expressions must be parenthesized before member or index access
- contains: property 'value' does not exist

### parenthesized instantiation expressions allow member access

> Parenthesized instantiation expressions can be used for member access.

```ds
function make<T>(value: T): T {
    return value
}

const next = (make<number>)(1);
next satisfies number;
```

### conflicting comptime arguments on member calls

> Comptime arguments cannot appear on both a member and its call.

```ds
interface Container<T> {
    map<U>(value: T): U
}

declare function getContainer(): Container<number>;

(getContainer().map<string>)<number>(1);
```

- comptime arguments specified on both member and call
