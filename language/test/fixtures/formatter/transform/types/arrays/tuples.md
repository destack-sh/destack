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

### tuple and array tuple stay distinct

Parenthesized tuples and bracket tuples preserve their spelling.

```ds
type Pair = (left: string, right: string)
type Args = [left: string, right: string]
```

```ds expected
type Pair = (left: string, right: string);
type Args = [left: string, right: string];
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

### nested conditional type

Nested conditional types preserve parentheses.

```ts:main.ts
type Nested<T> = T extends string ? (T extends "a" ? 1 : 2) : 3
```

```ts expected
type Nested<T> = T extends string ? (T extends "a" ? 1 : 2) : 3;
```
