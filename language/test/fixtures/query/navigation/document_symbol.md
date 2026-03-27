# Document Symbol

## Basic Declarations

### Functions and structs

Document symbols should return all top-level declarations in a file for the outline view.

```ds
struct Point {
    x: float32
    y: float32
}

function add(a: int32, b: int32): int32 {
    return a + b;
}

class Animal {
    name: string
}
```

The file has 3 declarations: struct `Point`, function `add`, and class `Animal`.
The snapshot asserts kinds plus ranges and selection ranges for each symbol.

```query document_symbols $0
Point(struct) range=1:1-4:2 selection=1:8-1:13
  x(field) range=2:5-2:15 selection=2:5-2:6
  y(field) range=3:5-3:15 selection=3:5-3:6
add(function) range=6:1-8:2 selection=6:10-6:13
Animal(class) range=10:1-12:2 selection=10:7-10:13
  name(field) range=11:5-11:17 selection=11:5-11:9
```

## Hierarchical Symbols

### Struct with fields

Struct members should appear as children of the struct.

```ds
struct Rectangle {
    width: float32
    height: float32

    area(): float32 {
        return this.width * this.height;
    }
}
```

The struct has fields `width` and `height`, and a method `area`.
The snapshot asserts the full outline tree.

```query document_symbols $0
Rectangle(struct) range=1:1-8:2 selection=1:8-1:17
  width(field) range=2:5-2:19 selection=2:5-2:10
  height(field) range=3:5-3:20 selection=3:5-3:11
  area(method) range=5:5-7:6 selection=5:5-5:9
```

### Class with members

Class members should appear as children of the class.
The snapshot asserts kinds and ranges for the class outline.

```ds
class Person {
    name: string
    age: int32

    greet(): string {
        return "Hello, " + this.name;
    }
}
```

```query document_symbols $0
Person(class) range=1:1-8:2 selection=1:7-1:13
  name(field) range=2:5-2:17 selection=2:5-2:9
  age(field) range=3:5-3:15 selection=3:5-3:8
  greet(method) range=5:5-7:6 selection=5:5-5:10
```

### Enum with fields

Enum fields should appear as children of the enum.
The snapshot asserts enum members with exact spans.

```ds
enum Color {
    Red,
    Green,
    Blue,
}
```

```query document_symbols $0
Color(enum) range=1:1-5:2 selection=1:6-1:11
  Red(enum_member) range=2:5-2:8 selection=2:5-2:8
  Green(enum_member) range=3:5-3:10 selection=3:5-3:10
  Blue(enum_member) range=4:5-4:9 selection=4:5-4:9
```

## Protocol Snapshots

### Snapshot includes kinds and ranges

Document symbols should support protocol shaped snapshots that include symbol kinds and ranges.

```ds
function outer(value: int32): int32 {
    function inner(delta: int32): int32 {
        return value + delta;
    }

    return inner(1);
}
```

The snapshot expectation asserts the entire symbol tree with kinds and line and column ranges.

```query document_symbols $0
outer(function) range=1:1-7:2 selection=1:10-1:15
inner(function) range=2:5-4:6 selection=2:14-2:19
```

### Snapshot for class members

Document symbols should snapshot classes with fields and methods.

```ds
class Box {
    value: int32

    get(): int32 {
        return this.value;
    }
}
```

The snapshot expectation asserts the class and its children with kinds and ranges.

```query document_symbols $0
Box(class) range=1:1-7:2 selection=1:7-1:10
  value(field) range=2:5-2:17 selection=2:5-2:10
  get(method) range=4:5-6:6 selection=4:5-4:8
```

### Snapshot for struct members

Document symbols should snapshot structs with fields and methods.

```ds
struct Pair {
    left: int32
    right: int32

    swap(): Pair {
        return Pair { left: this.right, right: this.left };
    }
}
```

The snapshot expectation asserts the struct and its children with kinds and ranges.

```query document_symbols $0
Pair(struct) range=1:1-8:2 selection=1:8-1:12
  left(field) range=2:5-2:16 selection=2:5-2:9
  right(field) range=3:5-3:17 selection=3:5-3:10
  swap(method) range=5:5-7:6 selection=5:5-5:9
```

### Snapshot for enum members

Document symbols should snapshot enums with enum members.

```ds
enum Direction {
    Up,
    Down,
}
```

The snapshot expectation asserts the enum and its members with kinds and ranges.

```query document_symbols $0
Direction(enum) range=1:1-4:2 selection=1:6-1:15
  Up(enum_member) range=2:5-2:7 selection=2:5-2:7
  Down(enum_member) range=3:5-3:9 selection=3:5-3:9
```

## Additional Kinds

### Namespace, interface, and type alias

Document symbols should include namespaces, interfaces, and type aliases with correct kinds.

```ds
namespace Utils {
    export function format(): string {
        return "ok";
    }
}

interface Drawable {
    draw(): void;
}

type UserId = string;
```

```query document_symbols $0
Utils(namespace) range=1:1-5:2 selection=1:11-1:16
format(function) range=2:5-4:6 selection=2:21-2:27
Drawable(interface) range=7:1-9:2 selection=7:11-7:19
  draw(method) range=8:5-8:17 selection=8:5-8:9
UserId(type_parameter) range=11:1-11:21 selection=11:6-11:12
```

## Empty Files

### Empty files return no document symbols

Document symbols should return no outline entries for empty files.

```ds
```

```query document_symbols $0
<none>
```

## Damaged Syntax

### Keep later symbols after malformed function declarations

Document symbols should still include later declarations after one malformed function head.

```ds
export function broken( {}

export function stableLater(): void {}

class StableBox {
    value: int32
}
```

```query document_symbols $0
broken(function) range=1:1-3:1 selection=1:17-1:23
stableLater(function) range=3:1-3:39 selection=3:17-3:28
StableBox(class) range=5:1-7:2 selection=5:7-5:16
  value(field) range=6:5-6:17 selection=6:5-6:10
```

### Keep later symbols after malformed call statements

Document symbols should still include later declarations after one malformed call statement.

```ds
broken(,

export function stableLater(): void {}

class StableBox {
    value: int32
}
```

```query document_symbols $0
stableLater(function) range=3:1-3:39 selection=3:17-3:28
StableBox(class) range=5:1-7:2 selection=5:7-5:16
  value(field) range=6:5-6:17 selection=6:5-6:10
```

### Keep later symbols after bare new recovery statements

Document symbols should still include later declarations after one bare `new` recovery statement.

```ds
new

export function stableLater(): void {}

class StableBox {
    value: int32
}
```

```query document_symbols $0
stableLater(function) range=3:1-3:39 selection=3:17-3:28
StableBox(class) range=5:1-7:2 selection=5:7-5:16
  value(field) range=6:5-6:17 selection=6:5-6:10
```

### Keep later symbols after throw recovery statements

Document symbols should still include later declarations after one recovered `throw` statement.

```ds
throw

export function stableLater(): void {}

class StableBox {
    value: int32
}
```

```query document_symbols $0
stableLater(function) range=3:1-3:39 selection=3:17-3:28
StableBox(class) range=5:1-7:2 selection=5:7-5:16
  value(field) range=6:5-6:17 selection=6:5-6:10
```

### Keep later symbols after yield star recovery statements

Document symbols should still include later declarations after one recovered `yield*` statement.

```ds
function* broken() {
    yield*
    const value = 1;
}

export function stableLater(): void {}

class StableBox {
    value: int32
}
```

```query document_symbols $0
broken(function) range=1:1-4:2 selection=1:11-1:17
stableLater(function) range=6:1-6:39 selection=6:17-6:28
StableBox(class) range=8:1-10:2 selection=8:7-8:16
  value(field) range=9:5-9:17 selection=9:5-9:10
```
