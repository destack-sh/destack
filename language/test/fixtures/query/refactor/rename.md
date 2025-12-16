# Rename

## Local Variables

### Rename local variable

Rename should update all occurrences of a symbol to the new name.

```ds
const foo = 1;
//    ^^^ target

const bar = foo + foo;
console.log(foo);
```

`foo` appears 4 times: its definition and 3 uses. Renaming to `baz` should update all 4 occurrences.

```query rename target "baz"
```

```expected:main
const baz = 1;

const bar = baz + baz;
console.log(baz);
```
