# Interface Index Signatures

## Index Signatures

### string index signature

String index signatures allow dictionary-like access.

```ds
interface Dict { [key: string]: number }
```

```ds expected
interface Dict {
    [key: string]: number;
}
```

### number index signature

Number index signatures allow array-like access.

```ds
interface ArrayLike { [index: number]: string }
```

```ds expected
interface ArrayLike {
    [index: number]: string;
}
```

### mixed index and properties

Index signatures can coexist with regular properties.

```ds
interface Dict { [key: string]: number; length: number }
```

```ds expected
interface Dict {
    [key: string]: number;
    length: number;
}
```
