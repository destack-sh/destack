# Tuple Types

## Tuple Types

### tuple type alias

Tuple types keep parentheses and commas.

```ds
type Point = (int32, int32)
```

```ds expected
type Point = (int32, int32);
```

### singleton tuple type alias

Singleton tuple types keep the required trailing comma.

```ds
type Single = (value: int32,)
```

```ds expected
type Single = (value: int32,);
```

### tuple type in function return

Tuple return types keep parentheses in signatures.

```ds
function point(): (x: int32, y: int32) { return (0, 0) }
```

```ds expected
function point(): (x: int32, y: int32) {
    return (0, 0);
}
```

### tuple and slice stay distinct

Parenthesized tuples and bracketed slices preserve their spelling.

```ds
type Pair = (left: string, right: string)
type Values = [string]
```

```ds expected
type Pair = (left: string, right: string);
type Values = [string];
```

### references in interface members

Ownership types keep their spelling inside interface members.

```ds
interface BufferView { borrow(): &readonly Buffer; take(value: ^Buffer): void; raw: *readonly Raw }
```

```ds expected
interface BufferView {
    borrow(): &readonly Buffer;
    take(value: ^Buffer): void;
    raw: *readonly Raw;
}
```

### multiline tuple ownership members

Tuple members with ownership types break one member per line when they exceed the width.

```ds line-width=56
type Handles = (borrowed: &readonly VeryLongBufferName, owned: ^VeryLongResultName, raw: *readonly VeryLongRawName)
```

```ds expected
type Handles = (
    borrowed: &readonly VeryLongBufferName,
    owned: ^VeryLongResultName,
    raw: *readonly VeryLongRawName,
);
```

### nested conditional type

Nested conditional types preserve parentheses.

```ds:main.ds
type Nested<T> = T extends string ? (T extends "a" ? 1 : 2) : 3
```

```ds expected
type Nested<T> = T extends string ? (T extends "a" ? 1 : 2) : 3;
```
