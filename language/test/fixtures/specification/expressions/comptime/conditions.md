# Comptime Conditions

## comptime conditions

### comptime condition selects expression type

> Comptime conditions can drive normal if-expressions.

```ds
const value: int32 = if (comptime true) { 1 } else { 2 };
value satisfies int32;
```

### comptime condition still type checks both branches

> Both branches of a comptime condition must type check.

```ds
const value: int32 = if (comptime true) { 1 } else { "nope" };
```

- contains: is not assignable

### comptime condition accepts type relations

> Comptime conditions can use type relations like `T extends U`.

```ds
function choose<T>(value: T): number {
    if (comptime T extends number) {
        return 1;
    }
    return 2;
}

choose<number>(1) satisfies number;
choose<string>("hi") satisfies number;
```

### comptime type condition narrows in true branch

> `comptime T extends U` narrows `T` to `T & U` in the true branch.

```ds
interface Named {
    name: string
}

function format<T>(value: T): string {
    if (comptime T extends Named) {
        value.name satisfies string;
        return value.name;
    }
    return "unknown";
}

format("ok");
format({ name: "Ada" });
```

### comptime type condition does not narrow in false branch

> The false branch does not gain members from a failed comptime relation.

```ds
interface Named {
    name: string
}

function format<T>(value: T): string {
    if (comptime T extends Named) {
        return value.name;
    }
    value.name;
    return "unknown";
}
```

- property 'name' does not exist on type T
