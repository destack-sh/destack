# Resolve Code Lens

## References

### Resolve a function reference count

Resolving a reference lens returns its reference count.

```ds main.ds
function ping(): void {}
         ^^^^ declaration

ping();
```

```query resolve_code_lens main.ds#declaration action=references
@resolve_code_lens.lens range=main.ds#declaration action=references symbol=main.ds#ping@1 count=1
```

### Resolve zero function references

Resolving an unused function lens reports zero references.

```ds main.ds
function unused(): void {}
         ^^^^^^ declaration
```

```query resolve_code_lens main.ds#declaration action=references
@resolve_code_lens.lens range=main.ds#declaration action=references symbol=main.ds#unused@1 count=0
```

## Implementations

### Resolve an interface implementation count

Resolving an implementation lens returns its direct implementation count.

```ds main.ds
interface Animal {
          ^^^^^^ declaration
    speak(): void;
}

class Dog implements Animal {
    speak(): void {}
}
```

```query resolve_code_lens main.ds#declaration action=implementations
@resolve_code_lens.lens range=main.ds#declaration action=implementations symbol=main.ds#Animal@1 count=1
```

### Resolve zero implementations

Resolving an unimplemented interface lens reports zero implementations.

```ds main.ds
interface Unimplemented {}
          ^^^^^^^^^^^^^ declaration
```

```query resolve_code_lens main.ds#declaration action=implementations
@resolve_code_lens.lens range=main.ds#declaration action=implementations symbol=main.ds#Unimplemented@1 count=0
```

## Tests

### Resolve a test lens beside a reference lens

The action selector distinguishes lenses that share one declaration range.

```ds main.ds
@test
function verify(): void {}
         ^^^^^^ declaration

verify();
```

```query resolve_code_lens main.ds#declaration action=run_test
@resolve_code_lens.lens range=main.ds#declaration action=run_test symbol=main.ds#verify@1 command=destack.runTest argument_0=main.ds#verify@1
```

### Resolve a debug action

Run and debug actions at the same declaration resolve independently.

```ds main.ds
@test
function verify(): void {}
         ^^^^^^ declaration
```

```query resolve_code_lens main.ds#declaration action=debug_test
@resolve_code_lens.lens range=main.ds#declaration action=debug_test symbol=main.ds#verify@1 command=destack.debugTest argument_0=main.ds#verify@1
```
