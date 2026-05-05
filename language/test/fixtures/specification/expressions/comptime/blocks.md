# Comptime Blocks

Top-level comptime blocks are static checks.

## module scope

### module comptime blocks assert static terms

> Top-level comptime blocks check static terms during compilation.

```ds
const size = comptime 4;

comptime {
    assert(size == 4);
}

size satisfies 4;
```

### module comptime blocks read imported terms

> Top-level comptime blocks can use imported static terms.

```ds:config.ds
export const size = comptime 4;
```

```ds:main.ds
import { size } from "./config";

comptime {
    assert(size == 4);
}

size satisfies 4;
```

### module comptime assertions can fail

> Failed comptime assertions are compile-time errors.

```ds
comptime {
    assert(false);
}
```

- contains: assertion

## exports

### comptime values initialize exports

> Exported constants can expose comptime values.

```ds
const computed = comptime {
    let base = 8;
    base + 4
};

export const size = computed;
size satisfies 12;
```
