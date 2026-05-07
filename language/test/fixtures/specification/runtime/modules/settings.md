# Module Settings

## local policy

### module block tightens memory policy

The body is checked as a static module record.

```ds
module {
    noManaged: true;
    noHeap: true;
}

function read(value: &[uint8]): uint8 {
    return value[0];
}
```

### module block tightens runtime policy

Runtime capability policy can also be tightened locally.

```ds
module {
    noRuntime: true;
    noExceptions: true;
    noImplicitDynamicDispatch: true;
}

function read(value: int32): int32 {
    return value;
}
```

## local providers

### module block selects tree and derive providers

Provider settings can reference imports because they are static terms.

```ds
import { HtmlTree } from "destack:ui/html";

module {
    tree: HtmlTree;
    derive: [Debug, Clone];
}

struct User {
    name: string;
}
```
