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

## nominality

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

### imported newtypes remain distinct

> Imported newtypes remain distinct even when their backing types match.

```ds:left.ds
export newtype UserId = int64;
```

```ds:right.ds
export newtype UserId = int64;
```

```ds:main.ds
import { UserId as LeftUserId } from "./left.ds";
import { UserId as RightUserId } from "./right.ds";

const id: LeftUserId = RightUserId(42);
```

- contains: not assignable
