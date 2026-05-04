# Drop

`drop(value)` explicitly ends ownership of an owned value.

## owned values

### drop consumes owned values

Dropping an owned value ends its ownership.

```ds
class File {
    handle: int32 = 0;
}

let file: ^File = new File();
drop(file);
```

### dropped values cannot be used again

Values are not usable after explicit drop.

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

### borrowed values cannot be dropped while the borrow is live

Dropping a value while a live borrow depends on it is invalid.

```ds
class File {
    handle: int32 = 0;
}

let file: ^File = new File();
let handle = &file.handle;
drop(file);
handle satisfies &int32;
```

- contains: borrowed
