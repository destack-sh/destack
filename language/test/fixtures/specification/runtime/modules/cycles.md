# Cycles

Module cycles must not require cross-module inference.

## inference

### unannotated cycles require type annotations

> Exported bindings do not infer through module cycles.

```ts:a.ts
import { y } from "./b";

export const x = y;
```

```ts:b.ts
import { x } from "./a";

export const y = x;
```

- contains: annotation

### partially annotated cycles still require local annotations

> One annotation does not infer the rest of a module cycle.

```ts:a.ts
import { y } from "./b";

export const x: number = y;
```

```ts:b.ts
import { x } from "./a";

export const y = x;
```

- contains: annotation

### fully annotated cycles are allowed

> Fully annotated exports do not require cycle inference.

```ts:a.ts
import { y } from "./b";

export const x: number = y;
```

```ts:b.ts
import { x } from "./a";

export const y: number = x;
```

### unannotated three-module cycles require type annotations

> Longer cycles follow the same local inference rule.

```ts:a.ts
import { y } from "./b";

export const x = y;
```

```ts:b.ts
import { z } from "./c";

export const y = z;
```

```ts:c.ts
import { x } from "./a";

export const z = x;
```

- contains: annotation

### three-module cycles need all inferred exports annotated

> A single annotation does not solve downstream exports in the cycle.

```ts:a.ts
import { y } from "./b";

export const x: number = y;
```

```ts:b.ts
import { z } from "./c";

export const y = z;
```

```ts:c.ts
import { x } from "./a";

export const z = x;
```

- contains: annotation

### namespace cycles require type annotations

> Namespace-based export cycles still require local annotations.

```ts:a.ts
import * as b from "./b";

export const x = b.y;
```

```ts:b.ts
import * as a from "./a";

export const y = a.x;
```

- contains: annotation

### namespace cycles need all inferred exports annotated

> One annotation does not infer namespace-imported cycle members.

```ts:a.ts
import * as b from "./b";

export const x: number = b.y;
```

```ts:b.ts
import * as a from "./a";

export const y = a.x;
```

- contains: annotation

### reexported cycles require type annotations

> Reexport chains inside a cycle still require local annotations.

```ts:a.ts
import { y } from "./forward";

export const x = y;
```

```ts:forward.ts
export { y } from "./b";
```

```ts:b.ts
import { x } from "./a";

export const y = x;
```

- contains: annotation

### reexported cycles need all inferred exports annotated

> One annotation does not solve an inferred export behind a reexport.

```ts:a.ts
import { y } from "./forward";

export const x: number = y;
```

```ts:forward.ts
export { y } from "./b";
```

```ts:b.ts
import { x } from "./a";

export const y = x;
```

- contains: annotation
