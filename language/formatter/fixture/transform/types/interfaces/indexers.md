# Interface Index Signatures

## Index Signatures

### string index signature

String index signatures allow dictionary-like access.

```tspp
interface Dict { [key: string]: number }
```

```tspp expected
interface Dict {
    [key: string]: number;
}
```

### number index signature

Number index signatures allow array-like access.

```tspp
interface ArrayLike { [index: number]: string }
```

```tspp expected
interface ArrayLike {
    [index: number]: string;
}
```

### mixed index and properties

Index signatures can coexist with regular properties.

```tspp
interface Dict { [key: string]: number; length: number }
```

```tspp expected
interface Dict {
    [key: string]: number;
    length: number;
}
```
