# Literal Precision

Literal widening, freshness, and const contexts.

## local bindings

### let scalar literals widen

`let` bindings without a constraining type widen scalar literals.

```ds
let value = 1;

value satisfies number;
```

### let scalar literals are not kept as literals

Widened `let` bindings are not assignable to the original literal type.

```ds
let value = 1;

value satisfies 1;
```

- contains: not assignable

### const scalar literals keep literal types

`const` bindings keep scalar literal types.

```ds
const value = 1;

value satisfies 1;
```

### const object members widen without const context

`const` does not implicitly freeze object members without a const context.

```ds
const config = { version: 1 };

config.version satisfies number;
```

### const object members are not kept as literals by default

Object members still widen without a const context.

```ds
const config = { version: 1 };

config.version satisfies 1;
```

- contains: not assignable

### const assertions suppress widening for object members

Const assertions suppress widening and preserve literal members.

```ds
const config = { version: 1 } as const;

config.version satisfies 1;
```

### const assertions preserve nested object members

Const assertions are deep and preserve nested literal members.

```ds
const config = { nested: { mode: "dev" } } as const;

config.nested.mode satisfies "dev";
```

### const assertions suppress widening for tuples

Const assertions preserve tuple literal members.

```ds
const pair = [1, 2] as const;

pair[0] satisfies 1;
pair[1] satisfies 2;
```

### const assertions preserve object literal array members

Const assertions preserve literal members in array elements.

```ds
const values = [{ kind: "a" }, { kind: "b" }] as const;

values[0].kind satisfies "a";
values[1].kind satisfies "b";
```

### const assertions preserve nested array members

Const assertions are deep for nested array literals.

```ds
const grid = [[1, 2]] as const;

grid[0][0] satisfies 1;
grid[0][1] satisfies 2;
```

### const assertions reject non literal conditional expressions

Const assertions are accepted on direct literal forms, not whole conditional expressions.

```ds
const value = (true ? 1 : 2) as const;
```

- contains: const assertions

### const arrays widen without const assertions

Array literals widen their element types without const assertions.

```ds
const pair = [1, 2];

pair[0] satisfies number;
```

### const arrays do not keep literal elements by default

Widened array elements are not assignable to the original literal element types.

```ds
const pair = [1, 2];

pair[0] satisfies 1;
```

- contains: not assignable

### annotations prevent widening but do not keep literal types

Contextual types prevent widening to primitives but do not keep literal types.

```ds
let value: 1 | 2 = 1;

value satisfies 1 | 2;
value satisfies 1;
```

- contains: not assignable

## imports

### exported const literals keep literal types across modules

Exported `const` literals keep their literal types across module boundaries.

```ds:values.ds
export const version = 1;
```

```ds:main.ds
import { version } from "./values.ds";

version satisfies 1;
```

### exported let literals widen across modules

Exported `let` literals widen to their primitive types across module boundaries.

```ds:values.ds
export let counter = 1;
```

```ds:main.ds
import { counter } from "./values.ds";

counter satisfies 1;
```

- contains: not assignable

### exported let literals still satisfy primitive types

Exported widened `let` literals still satisfy their primitive types.

```ds:values.ds
export let counter = 1;
```

```ds:main.ds
import { counter } from "./values.ds";

counter satisfies number;
```

### exported const assertions preserve literal members across modules

Const assertions on exports preserve literal members across module boundaries.

```ds:values.ds
export const config = { version: 1 } as const;
```

```ds:main.ds
import { config } from "./values.ds";

config.version satisfies 1;
```

### exported const objects still widen members without const assertions

Exported `const` object members widen without const assertions.

```ds:values.ds
export const config = { version: 1 };
```

```ds:main.ds
import { config } from "./values.ds";

config.version satisfies 1;
```

- contains: not assignable

### renamed re-export const literals keep literal types across modules

Renamed re-exports preserve exported const literal precision.

```ds:values.ds
export const version = 1;
```

```ds:index.ds
export { version as publicVersion } from "./values.ds";
```

```ds:main.ds
import { publicVersion } from "./index.ds";

publicVersion satisfies 1;
```

### export-star forwarded let literals still widen across modules

Export-star forwarding preserves widened `let` literal types.

```ds:values.ds
export let counter = 1;
```

```ds:index.ds
export * from "./values.ds";
```

```ds:main.ds
import { counter } from "./index.ds";

counter satisfies number;
```

### namespace imports preserve const assertion literal members

Namespace imports preserve const assertion literal member precision.

```ds:values.ds
export const config = { version: 1 } as const;
```

```ds:main.ds
import * as values from "./values.ds";

values.config.version satisfies 1;
```

## assignments

### assigning const scalar literals into let bindings widens

Fresh const scalar literals widen when assigned into mutable bindings.

```ds
const seed = 1;
let value = seed;

value satisfies number;
```

### assigning const scalar literals into let bindings does not keep literal

Mutable bindings do not preserve the original scalar literal.

```ds
const seed = 1;
let value = seed;

value satisfies 1;
```

- contains: not assignable

### assigning widened let scalars into const bindings keeps widened type

Const bindings do not re-narrow already widened sources.

```ds
let seed = "ready";
const value = seed;

value satisfies string;
```

## defaults and returns

### parameter defaults widen literal initializers

Parameter default literals widen to primitive parameter types in function bodies.

```ds
function readMode(mode = "dev") {
    mode satisfies string;
}
```

### parameter defaults do not keep literal initializers

Parameter default literals are not preserved as literal types by default.

```ds
function readMode(mode = "dev") {
    mode satisfies "dev";
}
```

- contains: not assignable

### function return inference widens literal returns

Function return inference widens unconstrained literal return expressions.

```ds
function makeMode() {
    return "dev";
}

const mode = makeMode();
mode satisfies string;
```

### function return inference does not keep literal returns

Unconstrained return inference does not preserve literal return values for plain function declarations.

```ds
function makeMode() {
    return "dev";
}

const mode = makeMode();
mode satisfies "dev";
```

- contains: not assignable

## nested boundaries

### nested function return inference widens literal returns

Nested function declarations widen unconstrained literal return values.

```ds
function outer() {
    function inner() {
        return "dev";
    }

    const mode = inner();
    mode satisfies string;
}
```

### nested function return inference does not keep literal returns

Nested function declarations do not preserve literal return values by default.

```ds
function outer() {
    function inner() {
        return "dev";
    }

    const mode = inner();
    mode satisfies "dev";
}
```

- contains: not assignable

### nested const to let binding still widens after inner usage

Using a const literal inside a nested function does not prevent later mutable widening.

```ds
const seed = "ready";

function observe() {
    seed satisfies "ready";
}

let widened = seed;
widened satisfies string;
```

### nested const to let binding does not keep literal after inner usage

Nested reads do not keep mutable bindings pinned to the original literal.

```ds
const seed = "ready";

function observe() {
    seed satisfies "ready";
}

let widened = seed;
widened satisfies "ready";
```

- contains: not assignable
