# Implementation

## Interface members

### Implementing methods

Goto implementation should return the methods that implement the selected interface member.

```ds:main.ds
interface /*impl_use*/Drawable {
   draw(): void;
}

class [|Circle|] implements Drawable {
   draw(): void {}
}

struct [|Rectangle|] implements Drawable {
   draw(): void {}
}
```

### Implementations across modules

Goto implementation should include cross-module implementers of the selected interface.

```ds:lib.ds
export interface /*impl_use*/Renderable {
   render(): void;
}
```

```ds:impl.ds
import type { Renderable } from "./lib.ds";

export class [|Sprite|] implements Renderable {
   render(): void {}
}

export struct [|Icon|] implements Renderable {
   render(): void {}
}
```

### Implementations through re-export chains

Goto implementation should still resolve implementers through type-only re-export chains.

```ds:types.ds
export interface /*impl_use*/Surface {
   draw(): void;
}
```

```ds:barrel.ds
export type { Surface } from "./types.ds";
```

```ds:impl.ds
import type { Surface } from "./barrel.ds";

export class [|Canvas|] implements Surface {
   draw(): void {}
}

export struct [|Poster|] implements Surface {
   draw(): void {}
}
```

### Subclasses across modules

Goto implementation should include subclasses of a selected base class across modules.

```ds:base.ds
export class /*impl_use*/Base {}
```

```ds:impl.ds
import { Base } from "./base.ds";

export class [|Derived|] extends Base {}
```
