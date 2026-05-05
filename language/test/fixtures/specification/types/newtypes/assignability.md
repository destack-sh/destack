# Newtype Assignability

Newtypes are nominal at assignment boundaries.

## backing

### backing values do not satisfy newtypes

```ds
newtype UserId = int64;

const id: UserId = 42;
```

- contains: not assignable

### newtypes do not satisfy backing types

```ds
newtype UserId = int64;

const raw: int64 = UserId(42);
```

- contains: not assignable

### backing projection is explicit

```ds
newtype UserId = int64;

const id = UserId(42);
const raw = id as int64;
raw satisfies int64;
```

## identity

### same newtype assigns to itself

```ds
newtype UserId = int64;

const source = UserId(42);
const target: UserId = source;
target satisfies UserId;
```

### distinct newtypes stay distinct

```ds
newtype UserId = int64;
newtype OrderId = int64;

const user = UserId(42);
const order: OrderId = user;
```

- contains: not assignable

### imported newtypes stay distinct

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
