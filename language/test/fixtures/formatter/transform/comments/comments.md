# Comments

Tests for comment formatting and preservation.

## Line Comments

### line comment on its own line

Comments on their own line are preserved in place.

```ds
// comment
const x = 1
```

The comment stays attached to the following statement.

```ds expected
// comment
const x = 1;
```

## Doc Comments

### doc comment before function

Doc comments using `///` are preserved before declarations.

```ds
/// This is a doc comment
function foo() { }
```

```ds expected
/// This is a doc comment
function foo() { }
```

## Comment Preservation

### preserves comment content exactly

Comment content is never modified by the formatter.

```ds
// TODO: fix this later
const x = 1
```

```ds expected
// TODO: fix this later
const x = 1;
```
