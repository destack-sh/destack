# Nominal Interfaces

## Nominal Interfaces

### newtype interface

Nominal interfaces use the `newtype interface` header.

```tspp
newtype interface Add<T, R = this> { add(other: T): R }
```

```tspp expected
newtype interface Add<T, R = this> {
    add(other: T): R;
}
```

### newtype marker interface

Marker interfaces can be empty.

```tspp
newtype interface Send { }
```

```tspp expected
newtype interface Send {}
```
