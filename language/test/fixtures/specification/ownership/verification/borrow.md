# Borrow Verification

## borrow conflicts

### mutable borrow conflicts with shared borrow

> Mutable borrows cannot overlap with shared borrows.

```ds native=true
struct Data {
    value: int32,
}

struct Container {
    data: Data,
}

function run(): void {
    let container = ^Container { data: Data { value: 1 } };
    let sharedRef = &container.data;
    let mutableRef = &mut container.data;
    sharedRef.value;
    mutableRef.value;
}
```

- contains: cannot borrow as mutable: already borrowed
