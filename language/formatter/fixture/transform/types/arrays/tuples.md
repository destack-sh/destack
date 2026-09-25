# Tuple Types

## Tuple Types

### tuple type alias

Tuple types keep parentheses and commas.

```tspp
type Point = (int32, int32)
```

```tspp expected
type Point = (int32, int32);
```

### singleton tuple type alias

Singleton tuple types keep the required trailing comma.

```tspp
type Single = (value: int32,)
```

```tspp expected
type Single = (value: int32,);
```

### tuple type in function return

Tuple return types keep parentheses in signatures.

```tspp
function point(): (x: int32, y: int32) { return (0, 0) }
```

```tspp expected
function point(): (x: int32, y: int32) {
    return (0, 0);
}
```

### tuple and slice stay distinct

Parenthesized tuples and bracketed slices preserve their spelling.

```tspp
type Pair = (left: string, right: string)
type Values = [string]
```

```tspp expected
type Pair = (left: string, right: string);
type Values = [string];
```

### references in interface members

Ownership types keep their spelling inside interface members.

```tspp
interface BufferView { borrow(): &readonly Buffer; take(value: ^Buffer): void; raw: *readonly Raw }
```

```tspp expected
interface BufferView {
    borrow(): &readonly Buffer;
    take(value: ^Buffer): void;
    raw: *readonly Raw;
}
```

### multiline tuple ownership members

Tuple members with ownership types break one member per line when they exceed the width.

```tspp line-width=56
type Handles = (borrowed: &readonly VeryLongBufferName, owned: ^VeryLongResultName, raw: *readonly VeryLongRawName)
```

```tspp expected
type Handles = (
    borrowed: &readonly VeryLongBufferName,
    owned: ^VeryLongResultName,
    raw: *readonly VeryLongRawName,
);
```

### nested conditional type

Nested conditional types preserve parentheses.

```tspp:main.tspp
type Nested<T> = T extends string ? (T extends "a" ? 1 : 2) : 3
```

```tspp expected
type Nested<T> = T extends string ? (T extends "a" ? 1 : 2) : 3;
```
