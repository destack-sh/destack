# Range Subscripts

A range inside a subscript selects a contiguous view.

## slices

### range indexing returns a slice view

A range subscript selects part of the underlying storage without copying.

```ds
declare const values: Slice<int32>;

const middle = values[1..4];
middle satisfies Slice<int32>;
```

### readonly range indexing preserves access

The slice keeps the receiver's access form.

```ds
declare const values: Slice<int32>;

const middle = (&readonly values)[1..4];
middle satisfies &readonly Slice<int32>;
```

### exclusive range indexing preserves access

Exclusive receivers produce exclusive slices.

```ds
declare const values: Slice<int32>;

const middle = (&exclusive values)[1..4];
middle satisfies &exclusive Slice<int32>;
```

### full range indexing returns the whole view

`[..]` views the entire sequence.

```ds
declare const values: Slice<int32>;

const all = values[..];
all satisfies Slice<int32>;
```

### one-sided range indexing returns a slice view

Open ends clamp to the sequence bounds.

```ds
declare const values: Slice<int32>;

const tail = values[2..];
const head = values[..4];
const prefix = values[..=4];

tail satisfies Slice<int32>;
head satisfies Slice<int32>;
prefix satisfies Slice<int32>;
```

### inclusive range indexing uses inclusive bounds

`..=` includes the final element.

```ds
declare const values: Slice<int32>;

const middle = values[1..=3];
middle satisfies Slice<int32>;
```

### range indexing accepts inferred endpoint types

Endpoint expressions infer like any other index.

```ds
declare const values: Slice<int32>;
declare const start: usize;
declare const end: usize;

const middle = values[start..end];
middle satisfies Slice<int32>;
```

## assignment

### range writes copy into the selected slice

Range assignment copies into an exclusive target range.

```ds
declare let values: Slice<int32>;

values[1..4] = [7, 8, 9];
```

### range writes accept readonly source slices

Copying only needs readonly access to the source.

```ds
declare let target: Slice<int32>;
declare const source: Slice<int32>;

target[1..4] = (&readonly source)[0..3];
```

### range writes require copyable elements

Range assignment copies elements, so owned elements need a different API.

```ds
class File {}

declare function openFile(path: string): ^File;

declare let files: Slice<^File>;

files[1..2] = [openFile("log.txt")];
```

- contains: no matching overload

### readonly range writes are rejected

Writing requires an exclusive target.

```ds
declare const values: &readonly Slice<int32>;

values[1..4] = [7, 8, 9];
```

- contains: no matching overload
