# Structs

> NOTE #Incomplete: implement/mdtest struct types

Data-oriented object types with fixed layout and value semantics.

## Coverage

- **Declaration**: `struct Point { x: float32, y: float32 }`
- **Construction**: `Point { x: 1, y: 2 }` or `new Point(1, 2)`
- **Nominal typing**: Must be explicitly constructed
- **Value semantics**: Passed by value (copied) by default
- **Embedding**: `...OtherStruct` to embed fields

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
