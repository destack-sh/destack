# Global Augmentation

## real globals

### imported global blocks contribute values

> A `global` block contributes real values when its module is in the graph.

```ds:globals.ds
global {
    const answer: int32 = 42;
}
```

```ds:main.ds
import "./globals.ds";

answer satisfies int32;
```

### duplicate global values are rejected

> Real global values cannot be declared twice.

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
import "./a.ds";
import "./b.ds";

answer satisfies int32;
```

- contains: duplicate

### configured global roots are visible without imports

> Configured global roots are part of the module closure.

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

## declare global

### global declarations are visible when imported

> Global augmentations become available once the declaring module is in the import graph.

```ds:globals.ds
declare global {
    interface GlobalThing {
        value: number
    }
}
```

```ds:main.ds
import "./globals.ds";

type Alias = GlobalThing;
```

### global declarations are not visible without import

> Global augmentations do not apply when the declaring module is not part of the import graph.

```ds:globals.ds
declare global {
    interface GlobalThing {
        value: number
    }
}
```

```ds:main.ds
type Alias = GlobalThing;
```

- contains: missing symbol

### global declarations merge across imports

> Multiple global augmentations merge into a single type.

```ds:a.ds
declare global {
    interface GlobalThing {
        value: number
    }
}
```

```ds:b.ds
declare global {
    interface GlobalThing {
        label: string
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

> Global class declarations merge static value members from every imported augmentation.

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

### global class merges tolerate namespace value augmentations

> Class and namespace global declarations can merge on one name without breaking instance typing.

```ds:a.ds
declare global {
    class GlobalWidget {
        ping(): number;
    }
}
```

```ds:b.ds
declare global {
    namespace GlobalWidget {
        export const tag: string;
    }
}
```

```ds:main.ds
import "./a.ds";
import "./b.ds";

declare const widget: GlobalWidget;
widget.ping() satisfies number;

GlobalWidget.tag satisfies string;
```

### global array augmentations preserve ambient array members

> Global `Array<T>` augmentations add members without removing ambient library members.

```ds:main.ds libs=es5
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

> Global declarations live in a separate scope from module declarations.

```ds:globals.d.ds
export {};

declare class Iterator<T> {
    next(value?: T): T;
}

interface IteratorConstructor {
    from<T>(value: T): Iterator<T>;
}

declare global {
    var Iterator: IteratorConstructor;
}
```

```ds:main.ds
import "./globals.d.ds";
```

### global augmentations do not conflict with module declarations

> Declaration files keep global augmentations separate from module-local declarations.

```ts:globals.d.ts
export {};

declare abstract class Iterator<T> {
    next(value?: T): T;
}

declare global {
    var Iterator: {
        new<T>(): Iterator<T>;
    };
}
```

```ts:main.ts
import "./globals.d.ts";
```

### global declarations in module bindings can use module scope

> Global augmentations declared inside module bindings can reference names from that module binding.

```ds:bindings.d.ds
export {};

declare module "foo" {
    type Local = number;

    global {
        interface GlobalThing {
            value: Local
        }
    }
}
```

```ds:main.ds
import "./bindings.d.ds";

type Alias = GlobalThing;
const value: Alias = { value: 1 };
```
