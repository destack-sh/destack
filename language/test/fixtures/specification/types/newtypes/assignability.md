# Newtype Assignability

Newtypes are nominal at assignment boundaries.

## backing

### backing values do not satisfy newtypes

The wrapper is nominal.

```ds
newtype UserId = int64;

const id: UserId = 42;
```

- contains: not assignable

### newtypes do not satisfy backing types

Unwrapping is explicit too.

```ds
newtype UserId = int64;

const raw: int64 = UserId(42);
```

- contains: not assignable

### backing casts are explicit

`as` opens the wrapper deliberately.

```ds
newtype UserId = int64;

const id = UserId(42);
const raw = id as int64;
raw satisfies int64;
```

## identity

### same newtype assigns to itself

Identity is by declaration.

```ds
newtype UserId = int64;

const source = UserId(42);
const target: UserId = source;
target satisfies UserId;
```

### distinct newtypes stay distinct

Sharing a backing type relates nothing.

```ds
newtype UserId = int64;
newtype OrderId = int64;

const user = UserId(42);
const order: OrderId = user;
```

- contains: not assignable

### imported newtypes stay distinct

Identity follows the declaration, not the name.

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
