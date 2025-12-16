# Find References

## Local Variables

### Find references to local variable

Find references should return all occurrences of a symbol, including its definition.

```ds
const foo = 1;
//    ^^^ def:foo

const bar = foo;
//          ^^^ use:foo
const baz = foo + foo;
```

`foo` is defined once and used 3 times (in `bar`, and twice in `baz`), so the total is 4 references.

```query find_references def:foo
4
```

## Functions

### Find references to function

Find references on a function should return its definition and all call sites.

```ds
function greet(name: string): string {
//       ^^^^^ def:greet
    return "Hello, " + name;
}

const a = greet("World");
//        ^^^^^ use:greet
const b = greet("Test");
```

`greet` is defined once and called twice (in `a` and `b`), so the total is 3 references.

```query find_references def:greet
3
```
