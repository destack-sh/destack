# Type Descriptors

Tests for `Type<T>` descriptors and `typeOf`.

## type as value

FUGU #Incomplete: type descriptors / RTTI inference (with inference)

## typeOf

### typeOf returns Type<T>

> typeOf returns a `Type<T>` descriptor for a value.

```ds
struct User { name: string }

const value = User { name: "Ada" };
const descriptor = typeOf(value);
descriptor satisfies Type<User>;
```
