# Nullish

`null`, `undefined`, and `void` stay explicit.

## null

### null has its own type

`null` is the unit type of `null`.

```ds
const value: null = null;
value satisfies null;
```

### null rejects non-nullish targets

Nothing is nullable implicitly.

```ds
const value: string = null;
```

- contains: not assignable

## undefined

### undefined has its own type

`undefined` is its own unit type.

```ds
const value: undefined = undefined;
value satisfies undefined;
```

### undefined rejects non-nullish targets

Nothing is optional implicitly.

```ds
const value: string = undefined;
```

- contains: not assignable

## void

### void marks no useful return value

`void` is for returns, not values.

```ds
function log(message: string): void {
    print(message);
}
```
