# Prepare Rename

## Local Variables

### Prepare rename local variable

Prepare rename should return the current name as placeholder for local variables.

```ds
const foo = 1;
//    ^^^ def:foo

const bar = foo + 2;
//          ^^^ ref:foo
```

When preparing to rename `foo` at its definition, we should get `foo` as the placeholder.

```query prepare_rename def:foo
foo
```

### Prepare rename at reference

We can also prepare rename from a reference site.

```ds
const value = 42;
//    ^^^^^ def:value

const result = value * 2;
//             ^^^^^ ref:value
```

Preparing rename at a reference should also return the current name.

```query prepare_rename ref:value
value
```

