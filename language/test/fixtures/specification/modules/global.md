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
