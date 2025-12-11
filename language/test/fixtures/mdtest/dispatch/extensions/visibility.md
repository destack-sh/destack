# Extension Visibility

Tests for extension visibility rules.

## Native Extensions

> Native extensions are defined in the same module as the type they extend.
> They are automatically visible wherever the type is used.

### native extension in same file

> Extension in same file as type is always visible.

```ds
struct Point { x: number, y: number }

extension Point {
    length(): number { return 0 }
}

declare function getPoint(): Point;

const p = getPoint();
const l: number = p.length();
```

### native extension on class

> Native extensions work on classes too.

```ds
class User { name: string }

extension User {
    greet(): string { return "" }
}

declare function getUser(): User;

const u = getUser();
const greeting: string = u.greet();
```

### native extension on interface

> Native extensions work on interfaces.

```ds
interface Shape {
    area(): number
}

extension Shape {
    describe(): string { return "" }
}

declare function getShape(): Shape;

const s = getShape();
const desc: string = s.describe();
```

## Anonymous Extensions

> Anonymous extensions on foreign types are only visible in the defining file.

### anonymous extension local to file

> Anonymous extension is visible in the same file where it's defined.

```ds
struct External {}

extension External {
    helper(): void {}
}

declare function getExternal(): External;

const e = getExternal();
e.helper();
```

## Named Extensions

> Named extensions must be explicitly imported to use.
> NOTE: Multi-file named extension tests require cross-module support.

### named extension defined locally

> Named extension in same file is visible without import.

```ds
struct Data {}

export extension DataHelpers: Data {
    process(): void {}
}

declare function getData(): Data;

const d = getData();
d.process();
```

## Multi-file Visibility (Placeholder)

> These tests require cross-module extension lookup to work.
> See is_extension_visible TODO for implementation status.

### native extension across modules

> Native extensions should be visible when the type is imported.
> NOTE: Currently blocked on cross-module TypeTable lookup.

```ds
// placeholder: single-file version
struct Vector2 { x: number, y: number }

extension Vector2 {
    length(): number { return 0 }
}

declare function getVector(): Vector2;

const v = getVector();
const l: number = v.length();
```
