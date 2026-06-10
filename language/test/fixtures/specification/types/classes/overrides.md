# Overrides

Override modifiers mark intentional class member overrides.

## overrides

### overrides require override

Shadowing without the keyword is an error.

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

The keyword states the intent.

```ds
class Base {
    greet(): void {}
}

class Derived extends Base {
    override greet(): void {}
}
```

### overrides require compatible methods

An override must satisfy the base signature.

```ds
class Base {
    greet(value: string): void {}
}

class Derived extends Base {
    override greet(value: number): void {}
}
```

- contains: not assignable
