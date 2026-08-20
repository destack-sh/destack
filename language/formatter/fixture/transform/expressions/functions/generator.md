# Generator Functions

## Generator Functions

### generator function

The `*` attaches to the `function` keyword with no space.

```ds
function  *  foo  (  )   {   }
```

```ds expected
function* foo() {}
```

### generator with yield

Single-statement loops stay on one line.

```ds
function* range(start: number, end: number) { for (let i = start; i < end; i++) { yield i } }
```

```ds expected
function* range(start: number, end: number) {
    for (let i = start; i < end; i++) {
        yield i;
    }
}
```

### async generator

String literals stay normalized inside async generators.

```ds
async function* items() { yield await fetch("a"); yield await fetch("b") }
```

```ds expected
async function* items() {
    yield await fetch("a");
    yield await fetch("b");
}
```
