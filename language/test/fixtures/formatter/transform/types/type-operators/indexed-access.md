# Indexed Access and Type Queries

## Indexed Access and Queries

### bracket tuple type

Bracket tuple types keep bracket syntax.

```ts:main.ts
type Pair = [T, boolean]
```

```ts expected
type Pair = [T, boolean];
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

### bracket tuple union

Empty and singleton bracket tuple types keep bracket syntax in unions.

```ts:main.ts
type Next<TNext> = [] | [TNext]
```

```ts expected
type Next<TNext> = [] | [TNext];
```

### bracket tuple rest element

Bracket tuple rest elements keep array suffixes on the rest type.

```ts:main.ts
type Requirements = [...PlatformCapability[]]
```

```ts expected
type Requirements = [...PlatformCapability[]];
```

### bracket tuple labeled rest payload

Labeled tuple rest elements keep the spread marker before the label.

```ts:main.ts
type RedisArgs = [...keys: RedisClient.KeyLike[], withscores: "WITHSCORES"]
```

```ts expected
type RedisArgs = [...keys: RedisClient.KeyLike[], withscores: "WITHSCORES"];
```

### bracket tuple optional label

Optional labeled tuple elements keep `?` on the label.

```ts:main.ts
type UpgradeOptions<WebSocketData> = [options?: {data?: undefined}, options: {data: WebSocketData}]
```

```ts expected
type UpgradeOptions<WebSocketData> = [
    options?: { data?: undefined },
    options: { data: WebSocketData },
];
```

### conditional bracket tuple optional label

Conditional tuple branches keep optional labels parseable after formatting.

```ts:main.ts
type UpgradeOptions<WebSocketData> = [WebSocketData] extends [undefined] ? [options?: {data?: undefined}] : [options: {data: WebSocketData}]
```

```ts expected
type UpgradeOptions<WebSocketData> = [WebSocketData] extends [undefined]
    ? [options?: { data?: undefined }]
    : [options: { data: WebSocketData }];
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
