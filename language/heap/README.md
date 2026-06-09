# Heap

`destack_heap` provides the worker-local and shared heaps used by the language runtime.
It covers page allocation, heap block metadata, and incremental garbage collection for both spaces.

## Testing

Run these from the repository root.

```sh
# focused local loop
cargo test -p destack_heap
cargo test -p destack_vm
cargo test -p destack_test --test optimize

# clean check
just language/check-quick

# exhaustive check
just language/check-full
```
