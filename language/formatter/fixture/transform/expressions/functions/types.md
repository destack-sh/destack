# Function Types

## Function Types

### function type alias

Function types keep parameter and return type spacing.

```tspp
type Predicate = (value: string) => boolean
```

```tspp expected
type Predicate = (value: string) => boolean;
```

### function type with receiver shorthand

Receiver shorthand is preserved in function types.

```tspp
type PlainVisitor = (this, value: Node) => void
type Visitor = (readonly this, value: Node) => void
type BorrowingVisitor = (&readonly this, value: Node) => void
```

```tspp expected
type PlainVisitor = (this, value: Node) => void;
type Visitor = (readonly this, value: Node) => void;
type BorrowingVisitor = (&readonly this, value: Node) => void;
```
