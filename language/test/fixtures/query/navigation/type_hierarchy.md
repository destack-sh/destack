# Type Hierarchy

## Supertypes

### Supertypes of a class

Supertypes should include the base class for a derived class.

```ds
class Base {}
//    ^^^^ def:Base

class Derived extends Base {}
//    ^^^^^^^ def:Derived
```

```query type_hierarchy def:Derived supertypes
main.ds:1:1-1:14 name=Base kind=class selection=main.ds:1:7-1:11
```

## Subtypes

### Subtypes of a class

Subtypes should include derived classes.

```ds
class Base {}
//    ^^^^ def:Base

class Derived extends Base {}
//    ^^^^^^^ def:Derived
```

```query type_hierarchy def:Base subtypes
main.ds:3:1-3:30 name=Derived kind=class selection=main.ds:3:7-3:14
```
