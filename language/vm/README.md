# VM

The VM is the MIR execution engine used for comptime evaluation, debugging, analysis, deterministic replay, and deopt fallback from native code.
It is fully featured for MIR _logic_, and it performs "external" work only via explicit platform bindings.
So, the VM by itself is just a pure resumable computation engine with serializable state, which is quite nice.

## Testing

Run these from the repository root.

```sh
# focused local loop
cargo test -p destack_vm
cargo test -p destack_test --test optimize

# clean check
just language/check-quick

# exhaustive check
just language/check-full
```
