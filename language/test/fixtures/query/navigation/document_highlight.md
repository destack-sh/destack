# Document Highlight

## Local Variables

### Highlight local variable occurrences

Document highlight should mark all occurrences of a symbol within the same file.

```ds
const foo = 1;
//    ^^^ def:foo

const bar = foo + foo;
//          ^^^ use:foo
```

`foo` appears 3 times: its definition and two uses in `foo + foo`.

```query document_highlight def:foo
3
```

## Functions

### Highlight function occurrences

Document highlight on a function should mark its definition and all call sites.

```ds
function add(x: int32, y: int32): int32 {
//       ^^^ def:add
    return x + y;
}

const result = add(1, 2) + add(3, 4);
```

`add` appears 3 times: its definition and two calls in the expression.

```query document_highlight def:add
3
```
