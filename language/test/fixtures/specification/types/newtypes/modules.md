# Newtypes Across Modules

## forwarding

### imported constructors preserve nominal newtype identity

> Imported newtype constructors produce values of the declared nominal type.

```ds:ids.ds
export newtype UserId = int64;
```

```ds:main.ds
import { UserId } from "./ids.ds";

const id = UserId(42);
id satisfies UserId;
```

### re-exported constructors preserve nominal newtype identity

> Re-exported newtype constructors preserve nominal identity through forwarding modules.

```ds:ids.ds
export newtype UserId = int64;
```

```ds:barrel.ds
export { UserId } from "./ids.ds";
```

```ds:main.ds
import { UserId } from "./barrel.ds";

const id = UserId(42);
id satisfies UserId;
```

## nominal identity across modules

### newtypes with the same backing type from different modules are distinct

> Newtypes declared in different modules remain nominally distinct.

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

- type UserId is not assignable to type UserId