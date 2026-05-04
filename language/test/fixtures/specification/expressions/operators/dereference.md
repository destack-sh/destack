# Dereference

Unary `*` uses `Deref` for receiver overloads.

## overloads

### dereference dispatches to Deref

> Unary `*` dispatches to `Deref` on the receiver.

```ds
struct Pointer { value: int }

extension of Pointer implements Deref<int> {
    deref(): int { return this.value }
}

declare function getPointer(): Pointer;

const pointer = getPointer();
const value = *pointer;
value satisfies int;
```

### dereference requires Deref

> Unary `*` requires a matching `Deref` implementation.

```ds
struct Pointer { value: int }

declare function getPointer(): Pointer;

const pointer = getPointer();
const value = *pointer;
value satisfies int;
```

- contains: no matching overload
