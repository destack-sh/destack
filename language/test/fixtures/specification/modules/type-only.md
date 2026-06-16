# Type Only

Type-only imports and exports never create runtime values.

## type-only exports

### type-only reexports stay type-only

Type-only reexports remain available in type positions.

```ds:types.ds
export type Options = { strict: boolean };
```

```ds:module-b.ds
export { type Options } from "./types.ds";
```

```ds:main.ds
import type { Options } from "./module-b.ds";

const options: Options = { strict: true };
options.strict satisfies boolean;
```

### type-only exports do not create values

Type-only exports do not provide runtime values.

```ds:types.ds
export type Options = { strict: boolean };
```

```ds:module-b.ds
export { type Options } from "./types.ds";
```

```ds:main.ds
import { Options } from "./module-b.ds";

const value = Options;
```

- contains: value


## type-only imports

### type-only imports do not create values

Type-only imports do not provide runtime values.

```ds:types.ds
export type Options = { strict: boolean };
```

```ds:module-b.ds
import type { Options } from "./types.ds";

export const value = Options;
```

```ds:main.ds
import { value } from "./module-b.ds";

value satisfies unknown;
```

- contains: type-only
