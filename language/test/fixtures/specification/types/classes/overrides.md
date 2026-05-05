# Overrides

Override modifiers mark intentional class member overrides.

## overrides

### overrides require override

```ds
class Base {
    greet(): void {}
}

class Derived extends Base {
    greet(): void {}
}
```

- contains: missing override modifier

### override marks intentional overrides

```ds
class Base {
    greet(): void {}
}

class Derived extends Base {
    override greet(): void {}
}
```

### overrides require compatible methods

```ds
class Base {
    greet(value: string): void {}
}

class Derived extends Base {
    override greet(value: number): void {}
}
```

- contains: not assignable
