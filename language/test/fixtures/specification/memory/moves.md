# Moves

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
