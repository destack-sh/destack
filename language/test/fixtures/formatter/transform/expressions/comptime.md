# Comptime Expressions

Tests for `comptime` expression formatting.

## Basic Comptime

### comptime expression

Comptime expressions keep `comptime` tight to the body.

```ds
const value = comptime 1 + 2 + 3
```

```ds expected
const value = comptime 1 + 2 + 3;
```

### comptime block expression

Comptime blocks format like other blocks.

```ds
const table = comptime { const x = 1; x + 1 }
```

```ds expected
const table = comptime {
    const x = 1;
    x + 1
};
```

### comptime if condition

Comptime conditions keep the keyword in the condition.

```ds
if (comptime Flag) { configure() }
```

```ds expected
if (comptime Flag) {
    configure()
}
```
