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

## Interfaces

### Supertypes of an implementing class

Supertypes should include implemented interfaces.

```ds
interface Animal {}
//        ^^^^^^ def:Animal

class Dog implements Animal {}
//    ^^^ def:Dog
```

```query type_hierarchy def:Dog supertypes
main.ds:1:1-1:20 name=Animal kind=interface selection=main.ds:1:11-1:17
```

### Subtypes of an interface

Subtypes should include direct implementing classes.

```ds
interface Animal {}
//        ^^^^^^ def:Animal

class Dog implements Animal {}
//    ^^^ def:Dog
```

```query type_hierarchy def:Animal subtypes
main.ds:3:1-3:31 name=Dog kind=class selection=main.ds:3:7-3:10
```

## Interface Extensions

### Supertypes of an extending interface

Supertypes should include extended interfaces.

```ds
interface Base {}
//        ^^^^ def:Base

interface Derived extends Base {}
//        ^^^^^^^ def:Derived
```

```query type_hierarchy def:Derived supertypes
main.ds:1:1-1:18 name=Base kind=interface selection=main.ds:1:11-1:15
```

### Subtypes of an interface include extensions

Subtypes should include interfaces that extend the target.

```ds
interface Base {}
//        ^^^^ def:Base

interface Derived extends Base {}
//        ^^^^^^^ def:Derived
```

```query type_hierarchy def:Base subtypes
main.ds:3:1-3:34 name=Derived kind=interface selection=main.ds:3:11-3:18
```

## Cross Module Classes

### Subtypes across modules

Subtypes should include classes declared in other modules.

```ds:lib.ds
export class Base {}
//           ^^^^ def:Base
```

```ds:impl.ds
import { Base } from "./lib.ds";

export class Derived extends Base {}
```

```ds:main.ds
import { Base } from "./lib.ds";
import { Derived } from "./impl.ds";

const _value: Base = new Derived();
```

```query type_hierarchy def:Base subtypes
impl.ds:3:1-3:37 name=Derived kind=class selection=impl.ds:3:14-3:21
```
