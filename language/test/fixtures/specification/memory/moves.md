# Moves

Owned values move, and a moved-from place is gone.

## move conflicts

### use after move is rejected

Values moved into owned parameters cannot be used afterwards.

```ds
struct Data {
    value: int32;
}

function consume(value: ^Data): void {
    value.value;
}

function run(): void {
    let data = ^Data { value: 1 };
    consume(data);
    data.value;
}
```

- contains: use of moved value

### moved value is allowed when not used again

Moving a value once is allowed when it is not used afterwards.

```ds
struct Data {
    value: int32;
}

function consume(value: ^Data): void {
    value.value;
}

function run(): void {
    let data = ^Data { value: 1 };
    consume(data);
}
```

### move in branch is rejected

Values moved on one control-flow path cannot be used afterwards.

```ds
struct Data {
    value: int32;
}

function consume(value: ^Data): void {
    value.value;
}

function run(flag: boolean): void {
    let data = ^Data { value: 1 };
    if (flag) {
        consume(data);
    }
    data.value;
}
```

- contains: may have been moved

### moving twice is rejected

Values cannot be moved more than once.

```ds
struct Data {
    value: int32;
}

function consume(value: ^Data): void {
    value.value;
}

function run(): void {
    let data = ^Data { value: 1 };
    consume(data);
    consume(data);
}
```

- contains: use of moved value

## borrows

### move while borrowed is rejected

Values cannot be moved while a borrow is active.

```ds
struct Data {
    value: int32;
}

struct Container {
    data: Data;
}

function consume(value: ^Container): void {
    value.data.value;
}

function run(): void {
    let container = ^Container { data: Data { value: 1 } };
    let borrow = &container.data;
    consume(container);
    borrow.value;
}
```

- contains: cannot move while borrowed

### borrow after move is rejected

Borrowing after a move is rejected.

```ds
struct Data {
    value: int32;
}

struct Container {
    data: Data;
}

function consume(value: ^Container): void {
    value.data.value;
}

function run(): void {
    let container = ^Container { data: Data { value: 1 } };
    consume(container);
    let borrow = &container.data;
    borrow.value;
}
```

- contains: use of moved value

## partiality

### field moves keep siblings available

Moving one owned field does not move disjoint fields.

```ds
class File {}

struct Pair {
    left: ^File;
    right: ^File;
}

declare function openFile(path: string): ^File;
declare function close(file: ^File): void;

function run(): void {
    let pair = ^Pair {
        left: openFile("left.txt"),
        right: openFile("right.txt"),
    };

    close(pair.left);
    pair.right satisfies ^File;
}
```

### partial moves reject whole value use

Using a whole value requires every moved field to be restored.

```ds
class File {}

struct Pair {
    left: ^File;
    right: ^File;
}

declare function openFile(path: string): ^File;
declare function close(file: ^File): void;
declare function archive(pair: ^Pair): void;

function run(): void {
    let pair = ^Pair {
        left: openFile("left.txt"),
        right: openFile("right.txt"),
    };

    close(pair.left);
    archive(pair);
}
```

- contains: partially moved

### assignment restores moved fields

Assigning a moved field makes the whole value available again.

```ds
class File {}

struct Pair {
    left: ^File;
    right: ^File;
}

declare function openFile(path: string): ^File;
declare function close(file: ^File): void;
declare function archive(pair: ^Pair): void;

function run(): void {
    let pair = ^Pair {
        left: openFile("left.txt"),
        right: openFile("right.txt"),
    };

    close(pair.left);
    pair.left = openFile("new-left.txt");
    archive(pair);
}
```

### variable element moves may overlap

A variable index may refer to any element of the same collection.

```ds
class File {}

declare function close(file: ^File): void;

function run(files: [^File; 2], index: uint): void {
    close(files[index]);
    close(files[0]);
}
```

- contains: may have been moved
