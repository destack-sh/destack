# Extension Resolution

Visible extensions participate in member lookup.

## overlap

### earlier extensions win duplicate methods

> When multiple visible extensions define the same method, the earlier extension is used.

```ds
struct Vector2 { x: number; y: number }

extension of Vector2 {
    process(): number { return 1 }
}

extension of Vector2 {
    process(): string { return "" }
}

declare function getVector(): Vector2;

const vector = getVector();
vector.process() satisfies number;
```
