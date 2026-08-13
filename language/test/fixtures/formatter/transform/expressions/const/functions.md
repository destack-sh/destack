# Const Functions

## Functions

### const function declaration

Const-only functions keep their phase marker before `function`.

```ds
const function layout<T, const Value: T>(): usize { return sizeOf<T>() + Value }
```

```ds expected
const function layout<T, const Value: T>(): usize {
    return sizeOf<T>() + Value;
}
```

### const generic parameter

Const value parameters live in the generic parameter list.

```ds
function select<const Name: string>(): string { return Name }
```

```ds expected
function select<const Name: string>(): string {
    return Name;
}
```
