# Overrides

Tests for override modifiers and `noImplicitOverride`.

## noImplicitOverride

### missing override on base member

> Members overriding a base class require the `override` modifier.

```ds:package.json
{ "name": "spec" }
```

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

- contains: missing override modifier

### override modifier on base member

> Members can opt in to `override` when they extend a base member.

```ds:package.json
{ "name": "spec" }
```

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

### override without base member

> `override` is rejected when no base member exists.

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "noImplicitOverride": false } }
```

```ds
class Base {
    greet(): void {}
}

class Derived extends Base {
    override hello(): void {}
}
```

- contains: override does not match a base member

### noImplicitOverride false allows missing override

> Missing `override` is allowed when `noImplicitOverride` is disabled.

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "noImplicitOverride": false } }
```

```ds
class Base {
    greet(): void {}
}

class Derived extends Base {
    greet(): void {}
}
```

### override requires compatible method signature

> Override members must remain compatible with the base member signature.

```ds:package.json
{ "name": "spec" }
```

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

- contains: not assignable
