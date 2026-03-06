# Goto Type Definition

## Aliases

### Type definition through an alias

Goto type definition should jump from an alias use site to the underlying type declaration.

```ds:main.ds
type /*type_def*/Point = number;
const current: /*type_use*/Point = 1;
```

