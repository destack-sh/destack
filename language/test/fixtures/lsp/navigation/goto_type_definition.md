# Goto Type Definition

## Aliases

### Type definition through an alias

Goto type definition should jump from an alias use site to the underlying type declaration.

```ds:main.ds
type /*type_def*/Point = number;
const current: /*type_use*/Point = 1;
```

### Type definition through a re-exported type

Goto type definition should follow a type-only re-export chain to the original declaration.

```ds:types.ds
export struct /*type_def*/Settings {
    enabled: bool
}
```

```ds:barrel.ds
export type { Settings } from "./types.ds";
```

```ds:main.ds
import type { Settings } from "./barrel.ds";

const current: /*type_use*/Settings = Settings { enabled: true };
```

### Type definition with mixed type and value imports

Goto type definition should still resolve the type when a value import with a nearby name is present.

```ds:types.ds
export struct /*type_def*/Point {
    value: int32
}
```

```ds:values.ds
export function point(): int32 {
    return 1;
}
```

```ds:main.ds
import type { Point } from "./types.ds";
import { point } from "./values.ds";

const current: /*type_use*/Point = Point { value: point() };
```

### Type definition for a default imported class

Goto type definition should resolve a default imported class alias back to the class declaration.

```ds:model.ds
export default class /*type_def*/Widget {
    value: int32;
}
```

```ds:main.ds
import WidgetModel from "./model.ds";

const current: /*type_use*/WidgetModel = new WidgetModel();
```
