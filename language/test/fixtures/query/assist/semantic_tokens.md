# Semantic Tokens

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
