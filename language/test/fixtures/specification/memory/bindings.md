# Bindings

## shared bindings

### shared const places the binding cell in shared space

> `shared const` declares a shared module binding and a shared value.

```ds
class Registry {}

shared const registry: Registry = new Registry();
registry satisfies shared Registry;
```

### local bindings can hold shared values

> A local binding can hold a shared value.

```ds
class Registry {}

const registry: shared Registry = new Registry();
registry satisfies shared Registry;
```
