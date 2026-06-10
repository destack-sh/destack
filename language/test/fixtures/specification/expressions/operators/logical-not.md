# Logical Not

`!` is not overloadable.

## expression

### logical not yields boolean

`!` produces boolean.

```ds
let value: boolean = !true;
```

### logical not requires boolean

There is no truthiness; control values must already be boolean.

```ds
const value = !42;
```

- contains: boolean
