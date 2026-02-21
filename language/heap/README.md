# Heap

`destack_heap` provides heap data structures and runtime values used by the language runtime stack.
It includes GC related internals, heap values, and string representations shared by runtime execution paths.

## Testing

Run these from the repository root.

### Quick local loop

```sh
cargo test -p destack_heap
```

### Runtime and VM coverage

```sh
cargo test -p destack_vm
cargo test -p destack_test --test optimize
```
