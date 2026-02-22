# Array Types Across Modules

## fixed-size arrays

### imported fixed-size arrays preserve their declared lengths

> Imported fixed-size arrays preserve declared lengths across module boundaries.

```ds:shapes.ds
export const pair: int32[2] = [1, 2];
```

```ds:main.ds
import { pair } from "./shapes.ds";

pair satisfies int32[2];
```

### re-exported fixed-size arrays preserve their declared lengths

> Re-exported fixed-size arrays preserve declared lengths through forwarding modules.

```ds:shapes.ds
export const pair: int32[2] = [1, 2];
```

```ds:barrel.ds
export { pair } from "./shapes.ds";
```

```ds:main.ds
import { pair } from "./barrel.ds";

pair satisfies int32[2];
```

## dynamic arrays

### imported dynamic arrays preserve element types

> Imported dynamic arrays preserve element types across modules.

```ds:data.ds
export const values: int32[] = [1, 2, 3];
```

```ds:main.ds
import { values } from "./data.ds";

values[0] satisfies int32;
```

### imported dynamic arrays reject incompatible assignment targets

> Imported dynamic arrays reject incompatible assignment targets.

```ds:data.ds
export const values: int32[] = [1, 2, 3];
```

```ds:main.ds
import { values } from "./data.ds";

const bad: string[] = values;
```

- contains: not assignable
