# Associated Imports

Associated members stay attached to the imported type.

## imports

### imports keep type members

> A type member can be used through an imported type.

```ds:layout.ds
export struct Box<T> {
    type Item = T;
    value: T;
}
```

```ds:main.ds
import { Box } from "./layout";

declare const value: Box<string>.Item;
value satisfies string;
```

### imports keep constant members

> A constant member can be used through an imported type.

```ds:layout.ds
export class FrameLayout {
    comptime const SegmentBytes: uint = 4096;
    type Segment = [uint8; this.SegmentBytes];
}
```

```ds:main.ds
import { FrameLayout } from "./layout";

declare const segment: FrameLayout.Segment;
segment satisfies [uint8; 4096];

const bytes = FrameLayout.SegmentBytes;
bytes satisfies 4096;
```

### imported requirements keep defaults

> Interface defaults still apply when the interface and implementor are imported from different modules.

```ds:profile.ds
export interface RegisterBlock {
    comptime const Width: uint = 4;
    type Bytes = [uint8; this.Width];
}
```

```ds:layout.ds
import { RegisterBlock } from "./profile";

export struct Status implements RegisterBlock {}
```

```ds:main.ds
import { Status } from "./layout";

declare const bytes: Status.Bytes;
bytes satisfies [uint8; 4];
```

## namespaces

### namespace imports keep associated members

> Associated members remain available through namespace imports.

```ds:layout.ds
export class Segment<Row> {
    comptime const Width: uint = Row extends string ? 8 : 4;
    type Lane = [uint8; this.Width];
}
```

```ds:main.ds
import * as layout from "./layout";

declare const lane: layout.Segment<string>.Lane;
lane satisfies [uint8; 8];
```
