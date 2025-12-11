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

nocheckin TODO #Incomplete: inherent / named extensions (within and across modules)