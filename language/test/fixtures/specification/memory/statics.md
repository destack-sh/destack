# Statics

## shared bindings

### shared const places the binding cell in shared space

`shared const` declares a shared module binding and a shared value.

```ds
class Registry {}

shared const registry: Registry = new Registry();
registry satisfies shared Registry;
```

### ordinary const keeps the binding local

Ordinary module bindings are Worker-local.

```ds
class Registry {}

const registry: Registry = new Registry();
registry satisfies Registry;
```

### shared value type keeps the binding local

`const value: shared T` is a local binding that holds a shared value.

```ds
class Registry {}

const registry: shared Registry = new Registry();
registry satisfies shared Registry;
```
