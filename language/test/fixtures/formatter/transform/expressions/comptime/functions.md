# Comptime Functions

## Functions

### comptime function declaration

Comptime-only functions keep their phase marker before `function`.

```ds
comptime function layout<T, comptime Value: T>(): usize { return sizeOf<T>() + Value }
```

```ds expected
comptime function layout<T, comptime Value: T>(): usize {
    return sizeOf<T>() + Value;
}
```

### comptime generic parameter

Comptime value parameters live in the generic parameter list.

```ds
function select<comptime Name: string>(): string { return Name }
```

```ds expected
function select<comptime Name: string>(): string {
    return Name;
}
```
