# Newtype Identity

## modules

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

- contains: not assignable
