# Cycles

Module cycles must not require cross-module inference.

## inference

### unannotated cycles require type annotations

> Exported bindings do not infer through module cycles.

```ds:a.ds
import { y } from "./b.ds";

export const x = y;
```

```ds:b.ds
import { x } from "./a.ds";

export const y = x;
```

- contains: annotation

### partially annotated cycles still require local annotations

> One annotation does not infer the rest of a module cycle.

```ds:a.ds
import { y } from "./b.ds";

export const x: number = y;
```

```ds:b.ds
import { x } from "./a.ds";

export const y = x;
```

- contains: annotation

### fully annotated cycles are allowed

> Fully annotated exports do not require cycle inference.

```ds:a.ds
import { y } from "./b.ds";

export const x: number = y;
```

```ds:b.ds
import { x } from "./a.ds";

export const y: number = x;
```

### unannotated three-module cycles require type annotations

> Longer cycles follow the same local inference rule.

```ds:a.ds
import { y } from "./b.ds";

export const x = y;
```

```ds:b.ds
import { z } from "./c.ds";

export const y = z;
```

```ds:c.ds
import { x } from "./a.ds";

export const z = x;
```

- contains: annotation

### three-module cycles need all inferred exports annotated

> A single annotation does not solve downstream exports in the cycle.

```ds:a.ds
import { y } from "./b.ds";

export const x: number = y;
```

```ds:b.ds
import { z } from "./c.ds";

export const y = z;
```

```ds:c.ds
import { x } from "./a.ds";

export const z = x;
```

- contains: annotation

### namespace cycles require type annotations

> Namespace-based export cycles still require local annotations.

```ds:a.ds
import * as b from "./b.ds";

export const x = b.y;
```

```ds:b.ds
import * as a from "./a.ds";

export const y = a.x;
```

- contains: annotation

### namespace cycles need all inferred exports annotated

> One annotation does not infer namespace-imported cycle members.

```ds:a.ds
import * as b from "./b.ds";

export const x: number = b.y;
```

```ds:b.ds
import * as a from "./a.ds";

export const y = a.x;
```

- contains: annotation

### reexported cycles require type annotations

> Reexport chains inside a cycle still require local annotations.

```ds:a.ds
import { y } from "./forward.ds";

export const x = y;
```

```ds:forward.ds
export { y } from "./b.ds";
```

```ds:b.ds
import { x } from "./a.ds";

export const y = x;
```

- contains: annotation

### reexported cycles need all inferred exports annotated

> One annotation does not solve an inferred export behind a reexport.

```ds:a.ds
import { y } from "./forward.ds";

export const x: number = y;
```

```ds:forward.ds
export { y } from "./b.ds";
```

```ds:b.ds
import { x } from "./a.ds";

export const y = x;
```

- contains: annotation
