# Structs

Data-oriented object types with fixed layout and value semantics.

## Coverage

- **Declaration**: `struct Point { x: float32, y: float32 }`
- **Construction**: `Point { x: 1, y: 2 }` or `new Point(1, 2)`
- **Tagged constructors**: `Point { x: 1, y: 2 }.length()`
- **Nominal typing**: Must be explicitly constructed
- **Assignability**: Structs and classes are not assignable to each other
- **Interfaces**: Structs can satisfy structural interfaces and `object`
- **Implements**: `struct Point implements Drawable`
- **Value semantics**: Passed by value (copied) by default
- **Identity**: `==` value comparison and `===` identity errors
- **Embedding**: `...OtherStruct` to embed fields
- **Decorators**: Field decorators on struct members

## Example

```ds
struct Point { x: float32, y: float32 }

let p: Point = Point { x: 1, y: 2 };  // ok
let p: Point = { x: 1, y: 2 };        // error: object literal is not Point

struct Entity {
    id: uint64,
    ...Transform  // embed Transform fields
}
```
