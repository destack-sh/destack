# Newtype Assignability

## backing type boundaries

### backing values are not assignable to newtypes

> Backing values do not implicitly coerce to nominal newtypes.

```ds
newtype UserId = int64;

const id: UserId = 42;
```

- contains: not assignable

### newtypes are not assignable to backing values

> Nominal newtypes do not implicitly coerce back to backing values.

```ds
newtype UserId = int64;

const raw: int64 = UserId(42);
```

- contains: not assignable

## nominal identity

### matching newtype aliases remain assignable to themselves

> Values are assignable within the same nominal newtype.

```ds
newtype UserId = int64;

const source = UserId(42);
const target: UserId = source;
```

### distinct newtypes with the same backing type are not assignable

> Distinct nominal newtypes remain incompatible even with shared backing types.

```ds
newtype UserId = int64;
newtype OrderId = int64;

const user = UserId(42);
const order: OrderId = user;
```

- contains: not assignable
