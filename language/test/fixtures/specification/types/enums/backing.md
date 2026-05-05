# Enum Backing Types

Enum backing values do not implicitly cross the enum boundary.

## integers

### integer enums do not coerce to integers

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

### integer enums cast to integers explicitly

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

### integers cast to enums explicitly

> Casting from the integer backing type to the enum is explicit.

```ds
enum Status {
    Active = 1
    Inactive = 2
}

const status = 1 as Status;
status satisfies Status;
```

### integers do not coerce to enums

> Integer values do not implicitly coerce to enums.

```ds
enum Status {
    Active = 1
    Inactive = 2
}

const status: Status = 1;
```

- contains: is not assignable

## strings

### string enums do not coerce to strings

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

### string enums cast to strings explicitly

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

### strings cast to enums explicitly

> Casting from string to the enum is explicit.

```ds
enum Flavor {
    Sweet = "sweet"
    Sour = "sour"
}

const flavor = "sweet" as Flavor;
flavor satisfies Flavor;
```

### enum members share one backing type

> Enum members must agree on backing type.

```ds
enum Mixed {
    First = 1
    Second = "two"
}
```

- contains: invalid enum backing type
