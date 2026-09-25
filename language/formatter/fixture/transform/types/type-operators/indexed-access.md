# Indexed Access and Type Queries

## Indexed Access and Queries

### tuple type

Tuple types use parentheses.

```tspp:main.tspp
type Pair = (T, boolean)
```

```tspp expected
type Pair = (T, boolean);
```

### slice type

TS++ slice types keep bracket syntax.

```tspp
type Values = [Value]
```

```tspp expected
type Values = [Value];
```

### readonly slice type

Slice element types keep readonly prefixes inside brackets.

```tspp
type Values = [readonly Value]
```

```tspp expected
type Values = [readonly Value];
```

### fixed array type

Fixed array types keep their length expression.

```tspp
type Bytes = [byte; 32]
type Lane<const N: uint> = [byte; N * 2]
```

```tspp expected
type Bytes = [byte; 32];
type Lane<const N: uint> = [byte; N * 2];
```

### tuple union

Empty and singleton tuple types keep their required punctuation in unions.

```tspp:main.tspp
type Next<TNext> = () | (TNext,)
```

```tspp expected
type Next<TNext> = () | (TNext,);
```

### tuple rest element

Tuple rest elements keep array suffixes on the rest type.

```tspp:main.tspp
type Requirements = (...HostAction[])
```

```tspp expected
type Requirements = (...HostAction[],);
```

### tuple labeled rest payload

Labeled tuple rest elements keep the spread marker after the label.

```tspp:main.tspp
type RedisArgs = (keys: ...RedisClient.KeyLike[], withscores: "WITHSCORES")
```

```tspp expected
type RedisArgs = (keys: ...RedisClient.KeyLike[], withscores: "WITHSCORES");
```

### tuple optional label

Optional labeled tuple elements keep `?` on the label.

```tspp:main.tspp
type UpgradeOptions<WebSocketData> = (options?: {data?: undefined}, options: {data: WebSocketData})
```

```tspp expected
type UpgradeOptions<WebSocketData> = (
    options?: { data?: undefined },
    options: { data: WebSocketData },
);
```

### conditional tuple optional label

Conditional tuple branches keep optional labels parseable after formatting.

```tspp:main.tspp
type UpgradeOptions<WebSocketData> = (WebSocketData,) extends (undefined,) ? (options?: {data?: undefined},) : (options: {data: WebSocketData},)
```

```tspp expected
type UpgradeOptions<WebSocketData> = (WebSocketData,) extends (undefined,)
    ? (options?: { data?: undefined },)
    : (options: { data: WebSocketData },);
```

### indexed access type

Indexed access types keep brackets tight.

```tspp
type Name = User["name"]
```

```tspp expected
type Name = User["name"];
```

### type query

Type queries keep a space after `typeof`.

```tspp
type Result = typeof someValue
```

```tspp expected
type Result = typeof someValue;
```
