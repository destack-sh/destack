# Global Augmentation

Globals are declared, never created implicitly.

## real globals

### configured global roots are visible without imports

Configured global roots expose shared runtime globals.

```ds:globals.ds
global {
    const runtimeName: string = "test";
}
```

```ds:main.ds
runtimeName satisfies string;
```

```json:destack.json
{ "compiler": { "globals": ["./globals.ds"] } }
```

### side-effect imports do not contribute globals

Bare imports evaluate modules without importing their global value declarations.

```ds:globals.ds
global {
    const answer: int32 = 42;
}
```

```ds:main.ds
import "./globals.ds";

answer satisfies int32;
```

- contains: missing symbol

### named imports do not contribute globals

Ordinary imports resolve exported bindings only.

```ds:globals.ds
export const value = 1;

global {
    const answer: int32 = 42;
}
```

```ds:main.ds
import { value } from "./globals.ds";

value satisfies int;
answer satisfies int32;
```

- contains: missing symbol

### namespace imports do not contribute globals

Namespace imports expose the target module namespace without importing globals.

```ds:globals.ds
export const value = 1;

global {
    const answer: int32 = 42;
}
```

```ds:main.ds
import * as globals from "./globals.ds";

globals.value satisfies int;
answer satisfies int32;
```

- contains: missing symbol

### reexports do not contribute globals

Reexport edges do not make target globals visible to importers.

```ds:globals.ds
export const value = 1;

global {
    const answer: int32 = 42;
}
```

```ds:index.ds
export { value } from "./globals.ds";
```

```ds:main.ds
import { value } from "./index.ds";

value satisfies int;
answer satisfies int32;
```

- contains: missing symbol

### type-only imports do not contribute globals

Type-only imports never affect global value visibility.

```ds:globals.ds
export type Value = int32;

global {
    const answer: int32 = 42;
}
```

```ds:main.ds
import type { Value } from "./globals.ds";

const value: Value = 1;
answer satisfies int32;
```

- contains: missing symbol

### duplicate global values are rejected

Real global values cannot be declared twice.

```ds:a.ds
global {
    const answer: int32 = 42;
}
```

```ds:b.ds
global {
    const answer: int32 = 7;
}
```

```ds:main.ds
answer satisfies int32;
```

```json:destack.json
{ "compiler": { "globals": ["./a.ds", "./b.ds"] } }
```

- contains: duplicate

## declare global

### global declarations are visible when imported

Global augmentations become available once the declaring module is in the import graph.

```ds:globals.ds
declare global {
    interface GlobalThing {
        value: number;
    }
}
```

```ds:main.ds
import "./globals.ds";

type Alias = GlobalThing;
```

### global declarations are not visible without import

Global augmentations do not apply when the declaring module is not part of the import graph.

```ds:globals.ds
declare global {
    interface GlobalThing {
        value: number;
    }
}
```

```ds:main.ds
type Alias = GlobalThing;
```

- contains: missing symbol

### global declarations merge across imports

Multiple global augmentations merge into a single type.

```ds:a.ds
declare global {
    interface GlobalThing {
        value: number;
    }
}
```

```ds:b.ds
declare global {
    interface GlobalThing {
        label: string;
    }
}
```

```ds:main.ds
import "./a.ds";
import "./b.ds";

const thing: GlobalThing = { value: 1, label: "ok" };
thing.label satisfies string;
```

### global class static members merge across imports

Global class declarations merge static value members from every imported augmentation.

```ds:a.ds
declare global {
    class GlobalBox {
        static left(): number;
    }
}
```

```ds:b.ds
declare global {
    class GlobalBox {
        static right(): string;
    }
}
```

```ds:main.ds
import "./a.ds";
import "./b.ds";

const left = GlobalBox.left();
left satisfies number;

const right = GlobalBox.right();
right satisfies string;
```

### global array augmentations preserve ambient array members

Global `Array<T>` augmentations add members without removing ambient library members.

```ds:main.ds
declare global {
    interface Array<T> {
        first(): T | undefined;
    }
}

const values = [1, 2, 3];
values.length satisfies number;
values.first() satisfies number | undefined;
```

### global declarations do not conflict with module declarations

Global declarations live in a separate scope from module declarations.

```ds:globals.ds
export {};

declare class Iterator<T> {
    next(value?: T): T;
}

interface IteratorConstructor {
    from<T>(value: T): Iterator<T>;
}

declare global {
    let Iterator: IteratorConstructor;
}
```

```ds:main.ds
import "./globals.ds";
```

### global augmentations do not conflict with module declarations

Declaration files keep global augmentations separate from module-local declarations.

```ds:globals.ds
export {};

declare abstract class Iterator<T> {
    next(value?: T): T;
}

declare global {
    let Iterator: {
        new <T>(): Iterator<T>;
    };
}
```

```ds:main.ds
import "./globals.ds";
```
