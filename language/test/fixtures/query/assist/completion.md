# Completion

## Basic Keywords

### Complete with keywords at statement position

At any statement position, completion should offer language keywords.

```ds
const foo = 1;
$0
```

```query completion $0
- function: keyword
- const: keyword
- struct: keyword
```

### Complete in empty file

Even in an empty file, completion should offer keywords.

```ds
$0
```

```query completion $0
- function: keyword
- const: keyword
```

## Type Position

### Complete primitives after colon

In type annotation position (after `:`), should show primitive types but NOT keywords.

```ds
const x: $0
```

```query completion $0
- int32: type_parameter
- string: type_parameter
- bool: type_parameter
! function: keyword
! const: keyword
```

### Complete after extends

After `extends` keyword, should show primitive types but NOT keywords.

```ds
class Child extends $0
```

```query completion $0
- int32: type_parameter
- string: type_parameter
! function: keyword
! const: keyword
```
