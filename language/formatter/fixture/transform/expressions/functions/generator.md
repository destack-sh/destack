# Generator Functions

## Generator Functions

### generator function

The `*` attaches to the `function` keyword with no space.

```tspp
function  *  foo  (  )   {   }
```

```tspp expected
function* foo() {}
```

### generator with yield

Single-statement loops stay on one line.

```tspp
function* range(start: number, end: number) { for (let i = start; i < end; i++) { yield i } }
```

```tspp expected
function* range(start: number, end: number) {
    for (let i = start; i < end; i++) {
        yield i;
    }
}
```

### async generator

String literals stay normalized inside async generators.

```tspp
async function* items() { yield await fetch("a"); yield await fetch("b") }
```

```tspp expected
async function* items() {
    yield await fetch("a");
    yield await fetch("b");
}
```
