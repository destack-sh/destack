# Literal Widening

Tests for TypeScript-style literal widening, freshness, and const contexts.

## Local bindings

### let scalar literals widen

`let` bindings without a constraining type widen scalar literals.

```ts
let value = 1;

value satisfies number;
```

### let scalar literals are not kept as literals

Widened `let` bindings are not assignable to the original literal type.

```ts
let value = 1;

value satisfies 1;
```

- contains: not assignable

### const scalar literals keep literal types

`const` bindings keep scalar literal types.

```ts
const value = 1;

value satisfies 1;
```

### const object members widen without const context

`const` does not implicitly freeze object members without a const context.

```ts
const config = { version: 1 };

config.version satisfies number;
```

### const object members are not kept as literals by default

Object members still widen without a const context.

```ts
const config = { version: 1 };

config.version satisfies 1;
```

- contains: not assignable

### const assertions suppress widening for object members

Const assertions suppress widening and preserve literal members.

```ts
const config = { version: 1 } as const;

config.version satisfies 1;
```

### const assertions suppress widening for tuples

Const assertions preserve tuple literal members.

```ts
const pair = [1, 2] as const;

pair[0] satisfies 1;
pair[1] satisfies 2;
```

### const arrays widen without const assertions

Array literals widen their element types without const assertions.

```ts
const pair = [1, 2];

pair[0] satisfies number;
```

### const arrays do not keep literal elements by default

Widened array elements are not assignable to the original literal element types.

```ts
const pair = [1, 2];

pair[0] satisfies 1;
```

- contains: not assignable

### annotations prevent widening but do not keep literal types

Contextual types prevent widening to primitives but do not keep literal types.

```ts
let value: 1 | 2 = 1;

value satisfies 1 | 2;
value satisfies 1;
```

- contains: not assignable

## Cross module surfaces

### exported const literals keep literal types across modules

Exported `const` literals keep their literal types across module boundaries.

```ts:values.ts
export const version = 1;
```

```ts:main.ts
import { version } from "./values";

version satisfies 1;
```

### exported let literals widen across modules

Exported `let` literals widen to their primitive types across module boundaries.

```ts:values.ts
export let counter = 1;
```

```ts:main.ts
import { counter } from "./values";

counter satisfies 1;
```

- contains: not assignable

### exported let literals still satisfy primitive types

Exported widened `let` literals still satisfy their primitive types.

```ts:values.ts
export let counter = 1;
```

```ts:main.ts
import { counter } from "./values";

counter satisfies number;
```

### exported const assertions preserve literal members across modules

Const assertions on exports preserve literal members across module boundaries.

```ts:values.ts
export const config = { version: 1 } as const;
```

```ts:main.ts
import { config } from "./values";

config.version satisfies 1;
```

### exported const objects still widen members without const assertions

Exported `const` object members widen without const assertions.

```ts:values.ts
export const config = { version: 1 };
```

```ts:main.ts
import { config } from "./values";

config.version satisfies 1;
```

- contains: not assignable

