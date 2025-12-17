# Semantic Tokens

Tests for LSP semantic tokens highlighting.

## Basic declarations

### Function declarations

Function declarations should be highlighted as functions with declaration modifier.

```ds
function foo() {}
```

```query semantic_tokens $0
foo: function [declaration]
```

### Class declarations

Class declarations should be properly highlighted.

```ds
class Animal {
    name: string
}
```

```query semantic_tokens $0
Animal: class [declaration]
string: type
```

### Struct declarations

Struct declarations should be highlighted as structs.

```ds
struct Point {
    x: float32,
    y: float32,
}
```

```query semantic_tokens $0
Point: struct [declaration]
float32: type
float32: type
```

### Interface declarations

Interface declarations should be highlighted.

```ds
interface Drawable {
    function draw(): void;
}
```

```query semantic_tokens $0
Drawable: interface [declaration]
void: type
```

### Namespace declarations

Namespace declarations should be highlighted.

```ds
namespace Utils {
    function helper(): void {}
}
```

```query semantic_tokens $0
Utils: namespace [declaration]
helper: function [declaration]
void: type
```

## Modifiers

### Async functions

Async functions should have the async modifier.

```ds
async function fetch(): void {}
```

```query semantic_tokens $0
fetch: function [declaration, async]
void: type
```

### Abstract classes

Abstract classes should have the abstract modifier.

```ds
abstract class Shape {}
```

```query semantic_tokens $0
Shape: class [declaration, abstract]
```

### Exported declarations

Exported declarations should have the definition modifier.

```ds
export function publicApi(): void {}
```

```query semantic_tokens $0
publicApi: function [declaration, definition]
void: type
```

## Enums

### Enum declarations

Enum declarations and their members should be highlighted.

```ds
enum Color {
    Red,
    Green,
    Blue,
}
```

```query semantic_tokens $0
Color: enum [declaration]
Red: enum_member [declaration]
Green: enum_member [declaration]
Blue: enum_member [declaration]
```

## Type parameters

### Generic functions

Type parameters should be highlighted as type parameters.

```ds
function identity<T>(value: T): T {
    return value;
}
```

```query semantic_tokens $0
identity: function [declaration]
T: type_parameter [declaration]
value: parameter [declaration]
value: variable
```

### Generic classes

Generic classes should have their type parameters highlighted.

```ds
class Container<T> {
    value: T
}
```

```query semantic_tokens $0
Container: class [declaration]
T: type_parameter [declaration]
T: variable
```

### Multiple type parameters

Multiple type parameters should all be highlighted.

```ds
function map<K, V>(key: K, value: V): void {}
```

```query semantic_tokens $0
map: function [declaration]
K: type_parameter [declaration]
V: type_parameter [declaration]
key: parameter [declaration]
value: parameter [declaration]
void: type
```

## Literals

### Number literals

Number literals should be highlighted.

```ds
function test(): int32 {
    return 42;
}
```

```query semantic_tokens $0
test: function [declaration]
int32: type
42: number
```

### Boolean literals

Boolean literals should be highlighted as numbers (keyword-like).

```ds
function test(): void {
    return true;
}
```

```query semantic_tokens $0
test: function [declaration]
void: type
true: number
```

## References

### Function references

References to functions should be highlighted as functions.

```ds
function foo(): void {}
function bar(): void {
    foo();
}
```

```query semantic_tokens $0
foo: function [declaration]
void: type
bar: function [declaration]
void: type
foo: function
```

### Class references

References to classes should be highlighted as classes.

```ds
class MyClass {}
function test(): void {
    new MyClass();
}
```

```query semantic_tokens $0
MyClass: class [declaration]
test: function [declaration]
void: type
MyClass: class
```
