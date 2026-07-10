# Indexed Access and Type Queries

## Indexed Access and Queries

### tuple type

Tuple types use parentheses.

```ds:main.ds
type Pair = (T, boolean)
```

```ds expected
type Pair = (T, boolean);
```

### slice type

Destack slice types keep bracket syntax.

```ds
type Values = [Value]
```

```ds expected
type Values = [Value];
```

### readonly slice type

Slice element types keep readonly prefixes inside brackets.

```ds
type Values = [readonly Value]
```

```ds expected
type Values = [readonly Value];
```

### fixed array type

Fixed array types keep their length expression.

```ds
type Bytes = [byte; 32]
type Lane<comptime N: uint> = [byte; N * 2]
```

```ds expected
type Bytes = [byte; 32];
type Lane<comptime N: uint> = [byte; N * 2];
```

### tuple union

Empty and singleton tuple types keep their required punctuation in unions.

```ds:main.ds
type Next<TNext> = () | (TNext,)
```

```ds expected
type Next<TNext> = () | (TNext,);
```

### tuple rest element

Tuple rest elements keep array suffixes on the rest type.

```ds:main.ds
type Requirements = (...HostAction[])
```

```ds expected
type Requirements = (...HostAction[],);
```

### tuple labeled rest payload

Labeled tuple rest elements keep the spread marker after the label.

```ds:main.ds
type RedisArgs = (keys: ...RedisClient.KeyLike[], withscores: "WITHSCORES")
```

```ds expected
type RedisArgs = (keys: ...RedisClient.KeyLike[], withscores: "WITHSCORES");
```

### tuple optional label

Optional labeled tuple elements keep `?` on the label.

```ds:main.ds
type UpgradeOptions<WebSocketData> = (options?: {data?: undefined}, options: {data: WebSocketData})
```

```ds expected
type UpgradeOptions<WebSocketData> = (
    options?: { data?: undefined },
    options: { data: WebSocketData },
);
```

### conditional tuple optional label

Conditional tuple branches keep optional labels parseable after formatting.

```ds:main.ds
type UpgradeOptions<WebSocketData> = (WebSocketData,) extends (undefined,) ? (options?: {data?: undefined},) : (options: {data: WebSocketData},)
```

```ds expected
type UpgradeOptions<WebSocketData> = (WebSocketData,) extends (undefined,)
    ? (options?: { data?: undefined },)
    : (options: { data: WebSocketData },);
```

### indexed access type

Indexed access types keep brackets tight.

```ds
type Name = User["name"]
```

```ds expected
type Name = User["name"];
```

### type query

Type queries keep a space after `typeof`.

```ds
type Result = typeof someValue
```

```ds expected
type Result = typeof someValue;
```
