# Enum Members

Enums can declare instance and static members.

## instance

### enum instances expose methods

Enum instances expose declared methods.

```ds
enum Status {
    Active = 1,
    Inactive = 2,

    isActive(): boolean {
        match (this) {
            Status.Active => true
            _ => false
        }
    }
}

const value = Status.Active.isActive();
value satisfies boolean;
```

## static

### enum statics expose values

Enum statics can expose shared values.

```ds
enum Status {
    Active = 1,
    Inactive = 2,

    static Default = Status.Active;
}

const value = Status.Default;
value satisfies Status;
```
