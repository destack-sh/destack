# Outline

## Declarations

### Outline top-level declarations

The outline follows declaration order and source ranges.

```ds main.ds
struct Point {
    x: float32;
    y: float32;
}

function add(left: int32, right: int32): int32 {
    return left + right;
}

class Animal {
    name: string;
}

const answer = 42;
```

```query outline main.ds
@outline.symbol depth=0 name=Point kind=struct range=main.ds:1:1-4:2 selection=main.ds:1:8-1:13
@outline.symbol depth=1 name=x kind=field detail=float32 range=main.ds:2:5-2:15 selection=main.ds:2:5-2:6
@outline.symbol depth=1 name=y kind=field detail=float32 range=main.ds:3:5-3:15 selection=main.ds:3:5-3:6
@outline.symbol depth=0 name=add kind=function detail="(left: int32, right: int32): int32" range=main.ds:6:1-8:2 selection=main.ds:6:10-6:13
@outline.symbol depth=0 name=Animal kind=class range=main.ds:10:1-12:2 selection=main.ds:10:7-10:13
@outline.symbol depth=1 name=name kind=field detail=string range=main.ds:11:5-11:17 selection=main.ds:11:5-11:9
@outline.symbol depth=0 name=answer kind=constant detail=42 range=main.ds:14:1-14:19 selection=main.ds:14:7-14:13
```

## Members

### Preserve hierarchical member order

Fields and methods immediately follow their owner.

```ds main.ds
struct Rectangle {
    width: float32;
    height: float32;

    area(): float32 {
        return this.width * this.height;
    }
}
```

```query outline main.ds
@outline.symbol depth=0 name=Rectangle kind=struct range=main.ds:1:1-8:2 selection=main.ds:1:8-1:17
@outline.symbol depth=1 name=width kind=field detail=float32 range=main.ds:2:5-2:19 selection=main.ds:2:5-2:10
@outline.symbol depth=1 name=height kind=field detail=float32 range=main.ds:3:5-3:20 selection=main.ds:3:5-3:11
@outline.symbol depth=1 name=area kind=method detail="(): float32" range=main.ds:5:5-7:6 selection=main.ds:5:5-5:9
```

### Distinguish member roles

Constructors, accessors, and static members retain their authored roles and signatures.

```ds main.ds
class Counter {
    static readonly zero: int32 = 0;
    value: int32;

    constructor(value: int32) {
        this.value = value;
    }

    get current(): int32 {
        return this.value;
    }

    set current(next: int32) {
        this.value = next;
    }

    static create(): Counter {
        return new Counter(0);
    }
}
```

```query outline main.ds
@outline.symbol depth=0 name=Counter kind=class range=main.ds:1:1-20:2 selection=main.ds:1:7-1:14
@outline.symbol depth=1 name=zero kind=field detail="static readonly int32" range=main.ds:2:5-2:36 selection=main.ds:2:21-2:25
@outline.symbol depth=1 name=value kind=field detail=int32 range=main.ds:3:5-3:17 selection=main.ds:3:5-3:10
@outline.symbol depth=1 name=constructor kind=constructor detail="(value: int32)" range=main.ds:5:5-7:6 selection=main.ds:5:5-5:16
@outline.symbol depth=1 name=current kind=property detail="get (): int32" range=main.ds:9:5-11:6 selection=main.ds:9:9-9:16
@outline.symbol depth=1 name=current kind=property detail="set (next: int32): void" range=main.ds:13:5-15:6 selection=main.ds:13:9-13:16
@outline.symbol depth=1 name=create kind=method detail="static (): Counter" range=main.ds:17:5-19:6 selection=main.ds:17:12-17:18
```

## Enum Members

### Outline enum members

Enum members remain children of their enum.

```ds main.ds
enum Color {
    Red,
    Green,
    Blue,
}
```

```query outline main.ds
@outline.symbol depth=0 name=Color kind=enum range=main.ds:1:1-5:2 selection=main.ds:1:6-1:11
@outline.symbol depth=1 name=Red kind=enum_member range=main.ds:2:5-2:8 selection=main.ds:2:5-2:8
@outline.symbol depth=1 name=Green kind=enum_member range=main.ds:3:5-3:10 selection=main.ds:3:5-3:10
@outline.symbol depth=1 name=Blue kind=enum_member range=main.ds:4:5-4:9 selection=main.ds:4:5-4:9
```

## Empty Modules

### Return an explicit empty outline

An empty module has no outline entries.

```ds main.ds
```

```query outline main.ds
@outline.none
```

## Types

### Outline structural types

Interfaces retain their members, and type aliases remain top-level symbols.

```ds main.ds
interface Drawable {
    draw(): void;
}

type UserId = string;
```

```query outline main.ds
@outline.symbol depth=0 name=Drawable kind=interface range=main.ds:1:1-3:2 selection=main.ds:1:11-1:19
@outline.symbol depth=1 name=draw kind=method detail="(): void" range=main.ds:2:5-2:17 selection=main.ds:2:5-2:9
@outline.symbol depth=0 name=UserId kind=type_alias detail=string range=main.ds:5:1-5:21 selection=main.ds:5:6-5:12
```

### Outline nominal types and extensions

Newtypes, nominal interfaces, and named extensions retain their distinct kinds.

```ds main.ds
newtype UserId = int64;

newtype interface Measure {
    abstract type Unit;
    abstract comptime const Scale: uint;
    measure(): float64;
}

extension Integer of int32 {
    doubled(): int32 {
        return this + this;
    }
}
```

```query outline main.ds
@outline.symbol depth=0 name=UserId kind=newtype detail=int64 range=main.ds:1:1-1:23 selection=main.ds:1:9-1:15
@outline.symbol depth=0 name=Measure kind=newtype_interface range=main.ds:3:1-7:2 selection=main.ds:3:19-3:26
@outline.symbol depth=1 name=Unit kind=associated_type range=main.ds:4:5-4:23 selection=main.ds:4:19-4:23
@outline.symbol depth=1 name=Scale kind=associated_const detail=uint range=main.ds:5:5-5:40 selection=main.ds:5:29-5:34
@outline.symbol depth=1 name=measure kind=method detail="(): float64" range=main.ds:6:5-6:23 selection=main.ds:6:5-6:12
@outline.symbol depth=0 name=Integer kind=extension range=main.ds:9:1-13:2 selection=main.ds:9:11-9:18
@outline.symbol depth=1 name=doubled kind=method detail="(): int32" range=main.ds:10:5-12:6 selection=main.ds:10:5-10:12
```

## Overloads

### Preserve authored overloads

Each overload remains a separate entry in source order.

```ds main.ds
declare function parse(value: string): int32;
declare function parse(value: int32): int32;
```

```query outline main.ds
@outline.symbol depth=0 name=parse kind=function detail="(value: string): int32" range=main.ds:1:1-1:45 selection=main.ds:1:18-1:23
@outline.symbol depth=0 name=parse kind=function detail="(value: int32): int32" range=main.ds:2:1-2:44 selection=main.ds:2:18-2:23
```

## Anonymous Owners

### Name an anonymous extension by its target

An anonymous extension remains visible without inventing a symbol identity.

```ds main.ds
extension of int32 {
    doubled(): int32 {
        return this + this;
    }
}
```

```query outline main.ds
@outline.symbol depth=0 name="extension of int32" kind=extension range=main.ds:1:1-5:2 selection=main.ds:1:14-1:19
@outline.symbol depth=1 name=doubled kind=method detail="(): int32" range=main.ds:2:5-4:6 selection=main.ds:2:5-2:12
```

## Omitted Symbols

### Omit imports and local bindings

The outline includes document declarations but not dependencies, parameters, or local bindings.

```ds main.ds
import { source } from "./library.ds";

function read(value: int32): int32 {
    const local = value;
    return local;
}

const exposed = 1;
```

```ds library.ds
export const source = 1;
```

```query outline main.ds
@outline.symbol depth=0 name=read kind=function detail="(value: int32): int32" range=main.ds:3:1-6:2 selection=main.ds:3:10-3:14
@outline.symbol depth=0 name=exposed kind=constant detail=1 range=main.ds:8:1-8:19 selection=main.ds:8:7-8:14
```
