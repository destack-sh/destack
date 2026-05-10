# Ownership Types

## Ownership Types

### borrowed reference type

Borrowed references keep the `&` operator tight to the type.

```ds
type Borrowed = &Buffer
```

```ds expected
type Borrowed = &Buffer;
```

### readonly borrowed reference type

Readonly borrows keep `&readonly` tight.

```ds
type Borrowed = &readonly Buffer
```

```ds expected
type Borrowed = &readonly Buffer;
```

### owned reference type

Owned references keep the `^` operator tight.

```ds
type Owned = ^Result
```

```ds expected
type Owned = ^Result;
```

### ownership type comments

Comments after ownership operators group the target type.

```ds
type Handles = (& /* borrowed */ Buffer, ^ /* owned */ Result, * /* pointer */ Raw)
```

```ds expected
type Handles = (&(/* borrowed */ Buffer), ^(/* owned */ Result), *(/* pointer */ Raw));
```

### readonly ownership type comments

Comments after readonly ownership prefixes group the target type.

```ds
type Handles = (&readonly /* borrowed */ Buffer, *readonly /* pointer */ Raw)
```

```ds expected
type Handles = (&readonly (/* borrowed */ Buffer), *readonly (/* pointer */ Raw));
```
