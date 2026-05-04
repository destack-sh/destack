# Match Patterns

Match patterns handle tuple, fixed array, ownership, union, range, and rest forms.

## tuple patterns

### match tuple patterns bind tuple elements

> Tuple patterns bind tuple elements by position.

```ds
declare const pair: (int32, int32);

match (pair) {
    (left, right) => {
        left satisfies int32;
        right satisfies int32;
    }
}
```

### match nested tuple patterns bind nested tuple elements

> Nested tuple patterns destructure recursively by position.

```ds
declare const pair: ((int32, int32), int32);

match (pair) {
    ((left, right), tail) => {
        left satisfies int32;
        right satisfies int32;
        tail satisfies int32;
    }
}
```

## fixed arrays

### match array patterns bind fixed array elements

> Fixed array patterns bind element types.

```ds
declare const pair: [int32; 2];

match (pair) {
    [left, right] => {
        left satisfies int32;
        right satisfies int32;
    }
}
```

## slices

### match slice patterns bind elements

> Slice patterns bind known positions and keep the tail as a slice.

```ds
declare const values: [int32];

match (values) {
    [first, ...rest] => {
        first satisfies int32;
        rest satisfies [int32];
    }
    _ => {}
}
```

## object patterns

### match object patterns bind named fields

> Object patterns bind named fields from structural objects.

```ds
type Config = { enabled: boolean, retries: int32 };

declare const config: Config;

match (config) {
    { enabled, retries } => {
        enabled satisfies boolean;
        retries satisfies int32;
    }
}
```

### match nested object patterns bind nested fields

> Nested object patterns bind nested object fields.

```ds
type Config = { runtime: { retries: int32 }, enabled: boolean };

declare const config: Config;

match (config) {
    { runtime: { retries }, enabled } => {
        retries satisfies int32;
        enabled satisfies boolean;
    }
}
```

### match object wildcard filters bind requested fields

> Object wildcard filters compose with named field bindings.

```ds
type Config =
    | { kind: "a", retries: int32, enabled: boolean }
    | { kind: "b", retries: int32, enabled: boolean };

declare const config: Config;

match (config) {
    { kind: _, retries } => {
        retries satisfies int32;
    }
}
```

## rest patterns

### match rest tuple patterns bind remaining elements

> Rest tuple patterns bind the remaining elements as a tuple.

```ds
declare const values: (int32, int32, int32);

match (values) {
    (first, ...rest) => {
        first satisfies int32;
        rest satisfies (int32, int32);
    }
}
```

### match rest array patterns bind remaining elements

> Fixed array rest patterns bind the remaining elements as a fixed array.

```ds
declare const values: [int32; 3];

match (values) {
    [first, ...rest] => {
        first satisfies int32;
        rest satisfies [int32; 2];
    }
}
```

### match rest object patterns bind remaining properties

> Rest object patterns bind the remaining properties.

```ds
type Config = { enabled: boolean, retries: int32 };

declare const config: Config;

match (config) {
    { enabled, ...rest } => {
        enabled satisfies boolean;
        rest satisfies { retries: int32 };
    }
}
```

## must patterns

### match must patterns bind non nullish values

> Must patterns bind non nullish values in match arms.

```ds
declare const value: int32 | null;

match (value) {
    x! => {
        x satisfies int32;
    }
    _ => {}
}
```

## ownership patterns

### match value patterns bind owned values

> Value patterns bind owned values without unwrapping.

```ds
declare const value: ^int32;

match (value) {
    ^x => {
        x satisfies ^int32;
    }
}
```

### match reference patterns bind references

> Reference patterns bind references without unwrapping.

```ds
declare const value: &int32;

match (value) {
    &x => {
        x satisfies &int32;
    }
}
```

## union patterns

### match union patterns cover multiple literals

> Union patterns match any of their literal values.

```ds
declare const value: 1 | 2 | 3;

match (value) {
    1 | 2 => "low"
    3 => "high"
}
```
