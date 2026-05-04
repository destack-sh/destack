# Enum Declarations

## backing types

### integer enum backing type

> Integer enums are nominal and do not implicitly coerce to integers.

```ds
enum Status {
    Active = 1
    Inactive = 2
}

const status = Status.Active;
const raw: int32 = status;
```

- contains: is not assignable

### integer enum explicit cast

> Casting to the integer backing type is explicit.

```ds
enum Status {
    Active = 1
    Inactive = 2
}

const status = Status.Active;
const raw = status as int32;
raw satisfies int32;
```

### integer to enum explicit cast

> Casting from the integer backing type to the enum is explicit.

```ds
enum Status {
    Active = 1
    Inactive = 2
}

const status = 1 as Status;
status satisfies Status;
```

### enums reject integer assignment

> Integer values do not implicitly coerce to enums.

```ds
enum Status {
    Active = 1
    Inactive = 2
}

const status: Status = 1;
```

- contains: is not assignable

### string enum backing type

> String enums are nominal and do not implicitly coerce to string.

```ds
enum Flavor {
    Sweet = "sweet"
    Sour = "sour"
}

const flavor = Flavor.Sweet;
const raw: string = flavor;
```

- contains: is not assignable

### string enum explicit cast

> Casting to string is explicit.

```ds
enum Flavor {
    Sweet = "sweet"
    Sour = "sour"
}

const flavor = Flavor.Sweet;
const raw = flavor as string;
raw satisfies string;
```

### string to enum explicit cast

> Casting from string to the enum is explicit.

```ds
enum Flavor {
    Sweet = "sweet"
    Sour = "sour"
}

const flavor = "sweet" as Flavor;
flavor satisfies Flavor;
```

### enums reject mixed values

> Enum members must agree on backing type.

```ds
enum Mixed {
    First = 1
    Second = "two"
}
```

- contains: invalid enum backing type

## members

### enum instance methods are available

> Enum instances expose declared methods.

```ds
enum Status {
    Active = 1
    Inactive = 2

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

### enum static fields are available

> Enum statics can expose shared values.

```ds
enum Status {
    Active = 1
    Inactive = 2

    static Default = Status.Active;
}

const value = Status.Default;
value satisfies Status;
```
