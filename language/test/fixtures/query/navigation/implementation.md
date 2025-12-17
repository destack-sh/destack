# Goto Implementation

## Basic interface implementation

### Find implementations of an interface

When cursor is on an interface name, find all types that implement it.

```ds
interface $0Drawable {
    draw(): void;
}

class Circle implements Drawable {
    draw(): void {}
}

struct Rectangle implements Drawable {
    draw(): void {}
}
```

```query implementation $0
2
```
