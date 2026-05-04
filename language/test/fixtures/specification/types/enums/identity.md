# Enum Identity

## modules

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
