# Readonly Types

## arrays

### readonly arrays accept mutable arrays

> Mutable arrays are assignable to readonly arrays.

```ds
declare let values: number[];
let frozen: readonly number[] = values;
frozen satisfies readonly number[];
```

### readonly arrays reject mutable assignment

> Readonly arrays are not assignable to mutable arrays.

```ds
declare let frozen: readonly number[];
let bad: number[] = frozen;
```

- contains: not assignable

## tuples

### readonly tuples accept mutable tuples

> Mutable tuples are assignable to readonly tuples.

```ds
type Pair = (number, string);
type ReadonlyPair = readonly (number, string);

declare let pair: Pair;
let frozen: ReadonlyPair = pair;
frozen satisfies ReadonlyPair;
```

### readonly tuples reject mutable assignment

> Readonly tuples are not assignable to mutable tuples.

```ds
type Pair = (number, string);
type ReadonlyPair = readonly (number, string);

declare let frozen: ReadonlyPair;
let bad: Pair = frozen;
```

- contains: not assignable

### readonly tuple elements do not imply readonly tuples

> Tuple element modifiers do not make the tuple readonly.

```ds
type ElemReadonly = (readonly int32, int32);

declare let values: ElemReadonly;
let arrayOk: int32[] = values;
arrayOk satisfies int32[];
```

### readonly tuple elements reject mutable element assignment

> Readonly tuple elements are not assignable to mutable tuple elements.

```ds
type ElemReadonly = (readonly int32, int32);
type Mutable = (int32, int32);

declare let values: ElemReadonly;
let bad: Mutable = values;
```

- contains: not assignable

### readonly tuples reject mutable array assignment

> Readonly tuples are not assignable to mutable arrays.

```ds
type ReadonlyPair = readonly (int32, int32);

declare let frozen: ReadonlyPair;
let bad: int32[] = frozen;
```

- contains: not assignable

## invalid readonly usage

### readonly operators require array or tuple targets

> Readonly type operators only apply to array and tuple types.

```ts
type T = string;
type Array<T> = T[];

type Bad1 = readonly string;
type Bad2 = readonly T;
type Bad3 = readonly readonly string[];
type Bad4 = readonly Array<string>;
```

- readonly type must target an array or tuple
