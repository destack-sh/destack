# Const Functions

## Functions

### const function declaration

Const-only functions keep their phase marker before `function`.

```tspp
const function layout<T, const Value: T>(): usize { return sizeOf<T>() + Value }
```

```tspp expected
const function layout<T, const Value: T>(): usize {
    return sizeOf<T>() + Value;
}
```

### const generic parameter

Const value parameters live in the generic parameter list.

```tspp
function select<const Name: string>(): string { return Name }
```

```tspp expected
function select<const Name: string>(): string {
    return Name;
}
```
