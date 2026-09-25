# Try Tail Values

## Try Tail Values

### try function tail expression

Try expressions in function tail position preserve try and catch values.

```tspp
function read(): number { try { value() } catch (error) { fallback(error) } }
```

```tspp expected
function read(): number {
    try {
        value()
    } catch (error) {
        fallback(error)
    }
}
```

### try in void function tail

Try and catch branches keep expression tails in void functions.

```tspp
function read(): void { try { value() } catch (error) { fallback(error) } }
```

```tspp expected
function read(): void {
    try {
        value()
    } catch (error) {
        fallback(error)
    }
}
```

### try function tail with finally

Finally blocks keep their expression tails like other try branches.

```tspp
function read(): number { try { value() } catch (error) { fallback(error) } finally { cleanup() } }
```

```tspp expected
function read(): number {
    try {
        value()
    } catch (error) {
        fallback(error)
    } finally {
        cleanup()
    }
}
```

### try function tail with explicit branch statements

Explicit semicolons inside try and catch branches are preserved.

```tspp
function read(): number { try { value(); } catch (error) { fallback(error); } }
```

```tspp expected
function read(): number {
    try {
        value();
    } catch (error) {
        fallback(error);
    }
}
```

### try tail comments

Comments before branch tail expressions stay in the branch block.

```tspp
function read(): number { try { // cached
value() } catch (error) { // fallback
fallback(error) } }
```

```tspp expected
function read(): number {
    try {
        // cached
        value()
    } catch (error) {
        // fallback
        fallback(error)
    }
}
```
