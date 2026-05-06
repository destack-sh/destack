# Drop

## explicit

### drop consumes owned values

`drop(value)` ends ownership at that point.

```ds
class File {
    handle: int32 = 0;
}

let file: ^File = new File();
drop(file);
```

### dropped values cannot be used

A dropped value cannot be used again.

```ds
class File {
    handle: int32 = 0;
}

let file: ^File = new File();
drop(file);
file.handle;
```

- contains: moved

## borrows

### drop before last borrow use is rejected

Owned storage cannot drop while a live borrow depends on it.

```ds
struct Point {
    x: int32;
}

let point = ^Point { x: 1 };
let x = &point.x;

drop(point);

x satisfies &int32;
```

- contains: borrowed

### drop after last borrow use is allowed

Owned storage can drop after dependent borrows are no longer live.

```ds
struct Point {
    x: int32;
}

let point = ^Point { x: 1 };
let x = &point.x;

x satisfies &int32;

drop(point);
```
