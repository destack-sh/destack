# Overrides

Override modifiers mark intentional class member overrides.

## overrides

### missing override on base member

> Members overriding a base class require the `override` modifier.

```ds
class Base {
    greet(): void {}
}

class Derived extends Base {
    greet(): void {}
}
```

- contains: missing override modifier

### override modifier on base member

> Members can opt in to `override` when they extend a base member.

```ds
class Base {
    greet(): void {}
}

class Derived extends Base {
    override greet(): void {}
}
```

### override requires compatible method signature

> Override members must remain compatible with the base member signature.

```ds
class Base {
    greet(value: string): void {}
}

class Derived extends Base {
    override greet(value: number): void {}
}
```

- contains: not assignable
