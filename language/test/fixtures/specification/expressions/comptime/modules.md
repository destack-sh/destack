# Comptime Modules

## module-level comptime blocks

### module-level comptime block is allowed

> Module-level comptime blocks are accepted declarations.

```ds
comptime {
    let size = 4;
    let _ = size + 1;
}

const value: int32 = 1;
value satisfies int32;
```

### module-level comptime block can appear after declarations

> Comptime blocks can appear after other top-level items.

```ds
const base: int32 = 2;

comptime {
    let value = base + 1;
    let _ = value;
}

const next = base + 1;
next satisfies int32;
```

### module-level comptime blocks can initialize exported constants

> Module-level comptime values can be forwarded through exported constants.

```ds
const computed = comptime {
    let base = 8;
    base + 4
};

export const size: int32 = computed;
size satisfies int32;
```

### module-level comptime blocks can read imported constants

> Module-level comptime blocks can use imported compile-time constants.

```ds:config.ds
export const base: int32 = 8;
```

```ds:main.ds
import { base } from "./config";

const computed = comptime {
    base + 4
};

computed satisfies int32;
```

### module-level comptime blocks reject runtime-only calls

> Module-level comptime blocks reject runtime-only computations.

```ds
function runtime_only(): int32 {
    4
}

const computed = comptime {
    runtime_only()
};
```

- contains: static expression
