# Comptime Blocks

Top-level comptime blocks run during compilation.

## module scope

### module comptime blocks run assertions

Assertions in a top-level comptime block fail the build, not the program.

```ds
const size = comptime 4;

comptime {
    assert(size == 4);
};

size satisfies 4;
```

### module comptime blocks read imported terms

Top-level comptime blocks can read imported constants.

```ds:config.ds
export const size = comptime 4;
```

```ds:main.ds
import { size } from "./config.ds";

comptime {
    assert(size == 4);
};

size satisfies 4;
```

### module comptime assertions can fail

Failed comptime assertions are compile-time errors.

```ds
comptime {
    assert(false);
};
```

- contains: assertion

## exports

### comptime values initialize exports

Exported constants can expose comptime values.

```ds
const computed = comptime {
    let base = 8;
    base + 4
};

export const size = computed;
size satisfies 12;
```
