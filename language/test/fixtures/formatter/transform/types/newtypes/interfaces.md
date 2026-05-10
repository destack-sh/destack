# Nominal Interfaces

## Nominal Interfaces

### newtype interface

Nominal interfaces use the `newtype interface` header.

```ds
newtype interface Add<T, R = this> { add(other: T): R }
```

```ds expected
newtype interface Add<T, R = this> {
    add(other: T): R;
}
```

### newtype marker interface

Marker interfaces can be empty.

```ds
newtype interface Send { }
```

```ds expected
newtype interface Send {}
```
