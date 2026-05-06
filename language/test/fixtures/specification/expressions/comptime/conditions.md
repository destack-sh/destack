# Comptime Conditions

Comptime conditions use compile-time boolean values for late branch elimination.

## conditions

### comptime conditions keep joined branch types

Comptime conditions type check like ordinary if-expressions.

```ds
const value = if (comptime true) { 1 } else { 2 };
value satisfies 1 | 2;
```

### comptime conditions check both branches

Both branches must type check against the surrounding expectation.

```ds
const value: int32 = if (comptime true) { 1 } else { "nope" };
```

- contains: is not assignable

### comptime conditions use static parameters

Static value parameters can choose lowering-time expression flow.

```ds
function choose<comptime UseFastPath: bool>(): int32 {
    if (comptime UseFastPath) {
        return 1;
    } else {
        return 2;
    }
}

choose<true>() satisfies int32;
choose<false>() satisfies int32;
```

### comptime conditions call functions with static inputs

Ordinary code can compute a late-eliminated condition from static inputs.

```ds
function isPowerOfTwo(value: uint): bool {
    if (value == 0) {
        return false;
    }

    let n = value;
    while (n > 1) {
        if (n % 2 != 0) {
            return false;
        }
        n /= 2;
    }

    return true;
}

function blockCost<comptime Width: uint>(): int32 {
    if (comptime isPowerOfTwo(Width)) {
        return 1;
    } else {
        return 2;
    }
}

blockCost<16>() satisfies int32;
blockCost<15>() satisfies int32;
```

### comptime conditions use inferred static parameters

Inferred static values can choose lowering-time expression flow after instantiation.

```ds
function length<T, comptime N: uint>(values: [T; N]): N {
    if (comptime N == 0) {
        return 0;
    } else {
        return N;
    }
}

const rgb: [uint8; 3] = [255, 128, 0];
const n = length(rgb);

n satisfies 3;
```

### comptime conditions reject runtime values

Dynamic parameters cannot choose comptime flow.

```ds
function choose(flag: bool): int32 {
    if (comptime flag) {
        return 1;
    } else {
        return 2;
    }
}
```

- contains: static expression
