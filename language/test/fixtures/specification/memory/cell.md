# Cell

Cells move mutation checking from places to values.

## cell

### cell supports interior mutation

`Cell` permits mutation through readonly access by storing its value in `UnsafeCell`.

```ds
import { Cell } from "destack:memory/cell";

let cell = Cell.new(1);
let view = &readonly cell;

view.set(2);
```

### cell get requires copyable values

Reading from a `Cell` copies the stored value.

```ds
import { Cell } from "destack:memory/cell";

let cell = Cell.new(1);
let value = cell.get();

value satisfies int32;
```

## ref cell

### ref cell guards release through using

`RefCell` moves borrow checking to runtime and releases guards through `Dispose`.

```ds
import { RefCell } from "destack:memory/cell";

let cell = RefCell.new(1);

using value = cell.borrowExclusive();
*value = 2;
```

### overlapping exclusive ref cell borrows are runtime failures

Static borrow checking permits this shape because `RefCell` enforces the rule dynamically.

```ds
import { RefCell } from "destack:memory/cell";

let cell = RefCell.new(1);

using first = cell.borrowExclusive();
let second = cell.tryBorrowExclusive();

second satisfies undefined;
```
