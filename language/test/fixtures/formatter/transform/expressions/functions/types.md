# Function Types

## Function Types

### function type alias

Function types keep parameter and return type spacing.

```ds
type Predicate = (value: string) => boolean
```

```ds expected
type Predicate = (value: string) => boolean;
```

### function type with receiver shorthand

Receiver shorthand is preserved in function types.

```ds
type PlainVisitor = (this, value: Node) => void
type Visitor = (readonly this, value: Node) => void
type BorrowingVisitor = (&readonly this, value: Node) => void
```

```ds expected
type PlainVisitor = (this, value: Node) => void;
type Visitor = (readonly this, value: Node) => void;
type BorrowingVisitor = (&readonly this, value: Node) => void;
```
