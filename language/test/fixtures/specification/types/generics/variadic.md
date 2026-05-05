# Variadic

## rest and spread inference

### variadic tuple tail extraction preserves literal heads

Tail extraction with a variadic tuple keeps the literal precision of preserved head elements.

```ds
type Head<T extends readonly unknown[]> = T extends readonly [infer H, ...unknown[]] ? H : never;

declare const value: Head<readonly ["a", "b", "c"]>;
value satisfies "a";
```

### variadic tuple concat preserves const tuple precision

Concatenating const tuples through variadic parameters retains fixed-length literal tuple precision.

```ds
declare function concat<T extends readonly unknown[], U extends readonly unknown[]>(a: T, b: U): [...T, ...U];

const value = concat([1, 2] as const, ["x"] as const);
value[0] satisfies 1;
value[2] satisfies "x";
```

### variadic tuple concat widens mutable arrays

Concatenating mutable arrays through variadic tuples widens to array-compatible element types.

```ds
declare function concat<T extends readonly unknown[], U extends readonly unknown[]>(a: T, b: U): [...T, ...U];

let left = [1, 2];
let right = ["x"];
const value = concat(left, right);

value[0] satisfies 1;
```

- contains: not assignable

### rest parameter inference preserves tuple element ordering

Rest-parameter inference over tuple inputs preserves element order in the inferred tuple result.

```ds
declare function collect<T extends readonly unknown[]>(...values: T): T;

const value = collect("a", 1, true);
value[0] satisfies "a";
value[1] satisfies 1;
value[2] satisfies true;
```
