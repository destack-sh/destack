# Module Declarations

## rules

### module decorators tighten memory policy

Module decorators apply to the current source module.

```ds
@noManaged
@noHeap
module {}

function read(value: &[uint8]): uint8 {
    return value[0];
}
```

### module decorators tighten runtime policy

Runtime capability policy can be tightened locally.

```ds
@noRuntime
@noImplicitDynamicDispatch
module {}

function read(value: int32): int32 {
    return value;
}
```

## metadata

### module metadata selects providers

Module metadata declarations are static terms.

```ds
import { Clone, Debug } from "destack:decorator";
import { HtmlTree } from "destack:ui/html";

module {
    const tree = HtmlTree;
    const derive = [Debug, Clone];
}

struct User {
    name: string;
}
```

### module metadata is visible through import.meta

```ds
module {
    const role = "server";
    const labels = {
        feature: ["search"],
    };
}

import.meta.role satisfies "server";
import.meta.labels.feature satisfies readonly ["search"];
```
