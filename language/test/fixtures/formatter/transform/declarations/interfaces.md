# Interface Declarations

Tests for interface declaration formatting.

## Basic Interfaces

### simple interface

```ds
interface   Foo   {   }
```

Empty interface bodies stay on one line.

```ds expected
interface Foo { }
```

### interface with extends

Multiple extended interfaces are separated by comma and space.

```ds
interface   Foo   extends   Bar  ,  Baz   {   }
```

```ds expected
interface Foo extends Bar, Baz { }
```
