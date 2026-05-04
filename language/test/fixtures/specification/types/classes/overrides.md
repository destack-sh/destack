# Overrides

Override modifiers and `noImplicitOverride`.

## noImplicitOverride

### missing override on base member

> Members overriding a base class require the `override` modifier.

```json:destack.json
{ "compiler": { "noImplicitOverride": true } }
```

```ds
class Base {
    greet(): void {}
}

class Derived extends Base {
    greet(): void {}
}
```

- missing override modifier

### override modifier on base member

> Members can opt in to `override` when they extend a base member.

```json:destack.json
{ "compiler": { "noImplicitOverride": true } }
```

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

```json:destack.json
{ "compiler": { "noImplicitOverride": true } }
```

```ds
class Base {
    greet(value: string): void {}
}

class Derived extends Base {
    override greet(value: number): void {}
}
```

- type (number): void is not assignable to type (string): void
