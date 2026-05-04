# Type Only

Type-only imports and exports never create runtime values.

## Type-only Exports

### type-only reexports stay type-only

Type-only reexports remain available in type positions.

```ts:types.ts
export type Options = { strict: boolean };
```

```ts:module-b.ts
export { type Options } from "./types";
```

```ts:main.ts
import type { Options } from "./module-b";

const options: Options = { strict: true };
options.strict satisfies boolean;
```

### type-only exports do not create values

> Type-only exports do not provide runtime values.

```ts:types.ts
export type Options = { strict: boolean };
```

```ts:module-b.ts
export { type Options } from "./types";
```

```ts:main.ts
import { Options } from "./module-b";

const value = Options;
```

- contains: value


## Type-only Imports

### type-only imports do not create values

> Type-only imports do not provide runtime values.

```ts:types.ts
export type Options = { strict: boolean };
```

```ts:module-b.ts
import type { Options } from "./types";

export const value = Options;
```

```ts:main.ts
import { value } from "./module-b";

value satisfies unknown;
```

- contains: type-only
