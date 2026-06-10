# Reference Counting

`Rc` and `Arc` add shared ownership on top of owned values.

## Rc

### Rc can create weak handles

`Rc` provides local shared ownership and `Weak` observes the allocation without keeping the value alive.

```ds
import * as rc from "destack:memory/rc";

struct User {
    name: string;
}

let owner = rc.Rc.new(^User { name: "Ada" });
let weak = owner.downgrade();
let upgraded = weak.upgrade();

upgraded satisfies rc.Rc<User> | undefined;
```

### Rc can build cyclic ownership

`Rc.newCyclic` initializes values that need a weak handle to themselves.

```ds
import * as rc from "destack:memory/rc";

struct Node {
    parent: rc.Weak<Node>;
}

let node = rc.Rc.newCyclic((self) => ^Node { parent: self });

node satisfies rc.Rc<Node>;
```

### Rc can expose unique access

Exclusive access through `Rc` is available only when the handle is unique.

```ds
import * as rc from "destack:memory/rc";

struct User {
    name: string;
}

let owner = rc.Rc.new(^User { name: "Ada" });
let user = owner.getExclusive();

user satisfies &exclusive User | undefined;
```

### Rc can recover unique ownership

`Rc.tryUnwrap` returns the owned value only when no other strong handle exists.

```ds
import { type Result } from "destack:error";
import * as rc from "destack:memory/rc";

struct User {
    name: string;
}

let owner = rc.Rc.new(^User { name: "Ada" });
let result = owner.tryUnwrap();

result satisfies Result<^User, rc.Rc<User>>;
```

## Arc

### Arc can create weak handles

`Arc` provides shared-space reference counting with the same weak ownership shape.

```ds
import * as arc from "destack:memory/arc";

struct User {
    name: string;
}

let owner = arc.Arc.new(^User { name: "Ada" });
let weak = owner.downgrade();
let upgraded = weak.upgrade();

upgraded satisfies arc.Arc<User> | undefined;
```

### Arc can build cyclic ownership

`Arc.newCyclic` provides the shared-space version of the same weak self-handle pattern.

```ds
import * as arc from "destack:memory/arc";

struct Node {
    parent: arc.Weak<Node>;
}

let node = arc.Arc.newCyclic((self) => ^Node { parent: self });

node satisfies arc.Arc<Node>;
```

### Arc can pin shared owners

Pinned `Arc` values keep the allocation stable for address-sensitive code.

```ds
import { type Pin } from "destack:memory";
import * as arc from "destack:memory/arc";

struct User {
    name: string;
}

let owner = arc.Arc.pin(^User { name: "Ada" });

owner satisfies Pin<arc.Arc<User>>;
```
