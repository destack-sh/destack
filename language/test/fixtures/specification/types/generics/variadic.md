# Variadic

## generic parameter lists

### variadic type constraints apply to each argument

A variadic type parameter constraint is checked once per supplied type argument.

```ds
type Row<...Fields: readonly (string | number)[]> = (...Fields);

declare const row: Row<"id", 42>;
row satisfies ("id", 42);
```

### variadic type constraints reject mismatched arguments

Each argument to a variadic type parameter has to satisfy the declared constraint.

```ds
type Row<...Fields: readonly string[]> = (...Fields);

declare const row: Row<"id", 42>;
```

- contains: constraint

### variadic value constraints apply to each argument

A variadic static value parameter constraint is checked once per supplied value argument.

```ds
type Shape<comptime ...Extents: readonly usize[]> = (...Extents);

declare const shape: Shape<64, 32>;
shape satisfies (64, 32);
```

### variadic value parameters bind static tuples

The name of a variadic static value parameter denotes the collected static tuple.

```ds
type TensorBuffer<comptime ...Extents: readonly usize[]> = {
    shape: Extents;
};

declare const buffer: TensorBuffer<64, 32>;
buffer.shape satisfies (64, 32);
```

## dynamic parameter lists

### rest parameter annotations describe the collected tuple

Dynamic rest parameters keep TypeScript's tuple-shaped rest annotation.

```ds
declare function collect<T: readonly unknown[]>(...values: T): T;

const values = collect("x", 1, true);
values satisfies readonly ("x", 1, true);
```

## rest and spread inference

### variadic tuple tail extraction preserves literal heads

Tail extraction with a variadic tuple keeps the literal precision of preserved head elements.

```ds
type Head<T: readonly unknown[]> = T extends readonly (infer H, ...unknown[]) ? H : never;

declare const value: Head<readonly ("a", "b", "c")>;
value satisfies "a";
```

### variadic tuple concat preserves const tuple precision

Concatenating const tuples through variadic parameters retains fixed-length literal tuple precision.

```ds
declare function concat<T: readonly unknown[], U: readonly unknown[]>(a: T, b: U): (...T, ...U);

const value = concat([1, 2] as const, ["x"] as const);
value[0] satisfies 1;
value[2] satisfies "x";
```

### variadic tuple concat widens mutable arrays

Concatenating mutable arrays through variadic tuples widens to array-compatible element types.

```ds
declare function concat<T: readonly unknown[], U: readonly unknown[]>(a: T, b: U): (...T, ...U);

let left = [1, 2];
let right = ["x"];
const value = concat(left, right);

value[0] satisfies 1;
```

- contains: not assignable

### rest parameter inference preserves tuple element ordering

Rest-parameter inference over tuple inputs preserves element order in the inferred tuple result.

```ds
declare function collect<T: readonly unknown[]>(...values: T): T;

const value = collect("a", 1, true);
value[0] satisfies "a";
value[1] satisfies 1;
value[2] satisfies true;
```
