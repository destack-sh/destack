# Type Parameter Modifiers

## const type parameters

### const modifiers are not allowed on type aliases

> Type alias parameters cannot use the const modifier.

```ts
type Bad<const T> = T;
```

- contains: invalid type parameter modifier
