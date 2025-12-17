# Class Declarations

Tests for class declaration formatting.

## Basic Classes

### simple class

Extra whitespace in class declarations should be normalized.

```ds
class   Foo   {   }
```

Empty class bodies stay on one line.

```ds expected
class Foo { }
```

### class with extends

The `extends` clause should have single spaces around it.

```ds
class   Foo   extends   Bar   {   }
```

```ds expected
class Foo extends Bar { }
```

### class with implements

Multiple implemented interfaces are separated by comma and space.

```ds
class   Foo   implements   Bar  ,  Baz   {   }
```

```ds expected
class Foo implements Bar, Baz { }
```

### class with generic

Generic type parameters have no internal spacing.

```ds
class   Foo  <  T  >   {   }
```

```ds expected
class Foo<T> { }
```
