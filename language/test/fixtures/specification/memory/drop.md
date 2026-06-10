# Drop

`Drop` runs when an owned value's lifetime ends.

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

## timing

### owned values drop after last use

Ordinary owned values do not stay alive just because the lexical scope continues.

```ds
class Buffer implements Drop {
    drop(): void {}
}

declare function openBuffer(): ^Buffer;
declare function read(buffer: &Buffer): void;
declare function unrelated(): void;

function run(): void {
    let buffer = openBuffer();

    read(&buffer);
    unrelated();
}
```

## borrows

### drop before last borrow use is rejected

An owned value cannot drop while a live borrow depends on it.

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

An owned value can drop after dependent borrows are no longer live.

```ds
struct Point {
    x: int32;
}

let point = ^Point { x: 1 };
let x = &point.x;

x satisfies &int32;

drop(point);
```

## partiality

### partial moves drop remaining fields

After a field move, only the remaining initialized owned fields are dropped.

```ds
class File {}

struct Row {
    first: ^File;
    second: ^File;
}

declare function openFile(path: string): ^File;
declare function take(file: ^File): void;

function run(): void {
    let row = ^Row {
        first: openFile("first.txt"),
        second: openFile("second.txt"),
    };

    take(row.first);
}
```

### drop types reject partial moves

Types that implement `Drop` cannot be partially moved.

```ds
class File {}

struct Row implements Drop {
    first: ^File;
    second: ^File;

    drop(): void {
        drop(this.first);
        drop(this.second);
    }
}

declare function openFile(path: string): ^File;
declare function take(file: ^File): void;

function run(): void {
    let row = ^Row {
        first: openFile("first.txt"),
        second: openFile("second.txt"),
    };

    take(row.first);
}
```

- contains: cannot partially move

### branch drops follow initialized paths

Each control-flow path drops the owned values still initialized on that path.

```ds
class File {}

declare function openFile(path: string): ^File;
declare function take(file: ^File): void;

function run(flag: boolean): void {
    let file = openFile("log.txt");

    if (flag) {
        take(file);
    }
}
```
