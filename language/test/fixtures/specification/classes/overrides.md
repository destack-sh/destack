# Overrides

Tests for override modifiers and `noImplicitOverride`.

## noImplicitOverride

### missing override on base member

> Members overriding a base class require the `override` modifier.

```ds:package.json
{ "name": "spec" }
```

```ds:dsconfig.json
{ "compilerOptions": { "noImplicitOverride": true } }
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

```ds:dsconfig.json
{ "compilerOptions": { "noImplicitOverride": true } }
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

```ds:dsconfig.json
{ "compilerOptions": { "noImplicitOverride": false } }
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
