# Enum Nominality

Enums are distinct by declaration, not by shape.

## imports

### imported enums with matching members remain distinct

Enums declared in different modules remain distinct even when their members match.

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

- contains: not assignable
