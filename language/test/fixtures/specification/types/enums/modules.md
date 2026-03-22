# Enum Types Across Modules

## forwarding

### imported enum members preserve the enum type

> Imported enum members preserve their enum type across module boundaries.

```ds:status.ds
export enum Status {
    Active = 1
    Inactive = 2
}
```

```ds:main.ds
import { Status } from "./status.ds";

const value = Status.Active;
value satisfies Status;
```

### re-exported enum members preserve the enum type

> Re-exported enum members preserve enum identity through forwarding modules.

```ds:status.ds
export enum Status {
    Active = 1
    Inactive = 2
}
```

```ds:barrel.ds
export { Status } from "./status.ds";
```

```ds:main.ds
import { Status } from "./barrel.ds";

const value = Status.Active;
value satisfies Status;
```

## nominal identity

### enums with identical members from different modules remain distinct

> Enums declared in different modules remain nominally distinct even when members match.

```ds:left.ds
export enum Status {
    Active = 1
    Inactive = 2
}
```

```ds:right.ds
export enum Status {
    Active = 1
    Inactive = 2
}
```

```ds:main.ds
import { Status as LeftStatus } from "./left.ds";
import { Status as RightStatus } from "./right.ds";

const value: LeftStatus = RightStatus.Active;
```

- type Status is not assignable to type Status