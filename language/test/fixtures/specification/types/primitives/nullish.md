# Nullish

`null`, `undefined`, and `void` stay explicit.

## null

### null has its own type

```ds
const value: null = null;
value satisfies null;
```

### null rejects non-nullish targets

```ds
const value: string = null;
```

- contains: not assignable

## undefined

### undefined has its own type

```ds
const value: undefined = undefined;
value satisfies undefined;
```

### undefined rejects non-nullish targets

```ds
const value: string = undefined;
```

- contains: not assignable

## void

### void marks no useful return value

```ds
function log(message: string): void {
    print(message);
}
```
