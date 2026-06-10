# Object Type

`object` is not a TS++ type.
Use `unknown`, structural object shapes, interfaces, `Record<K, V>`, or `Dynamic<T>`.

## removed type

### object annotations are rejected

The `object` type has no place in Destack's model.

```ds
const x: object = {};
```

- contains: unsupported type: object

### object parameters are rejected

Use a structural shape or `unknown` instead.

```ds
function id(value: object) {
    value
}
```

- contains: unsupported type: object
