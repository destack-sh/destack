# Global Augmentation

## Declare Global

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
