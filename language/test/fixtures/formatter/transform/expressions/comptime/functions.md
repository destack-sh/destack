# Comptime Functions

## Functions

### comptime function declaration

Comptime-only functions keep their phase marker before `function`.

```ds
comptime function layout<T>(comptime value: T): usize { return sizeOf<T>() + value }
```

```ds expected
comptime function layout<T>(comptime value: T): usize {
    return sizeOf<T>() + value;
}
```

### comptime pattern parameter

Comptime dynamic parameters keep their marker before the binding pattern.

```ds
function select(comptime { name }: Config): string { return name }
```

```ds expected
function select(comptime { name }: Config): string {
    return name;
}
```
