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

## Empty Hierarchy

### Root classes have no supertypes

Type hierarchy supertypes should be empty for root classes.

```ds
class $0Root {}
```

```query type_hierarchy $0 supertypes
<none>
```

### Leaf classes have no subtypes

Type hierarchy subtypes should be empty for leaf classes.

```ds
class Root {}

class $0Leaf extends Root {}
```

```query type_hierarchy $0 subtypes
<none>
```

## Re-Export Chains

### Subtypes through multi-hop type re-exports

Type hierarchy should preserve subtype edges through type-only re-export chains.

```ds:types.ds
export interface Renderable {}
//              ^^^^^^^^^^ def:Renderable
```

```ds:barrel_a.ds
export type { Renderable as Surface } from "./types.ds";
```

```ds:barrel_b.ds
export type { Surface } from "./barrel_a.ds";
```

```ds:impl.ds
import type { Surface } from "./barrel_b.ds";

export class Sprite implements Surface {}
```

```query type_hierarchy def:Renderable subtypes
impl.ds:3:1-3:42 name=Sprite kind=class selection=impl.ds:3:14-3:20
```

## Damaged Syntax

### Keep supertypes after malformed declarations

Type hierarchy should still resolve later supertypes after one malformed declaration.

```ds
export function broken( {}

class Base {}
//    ^^^^ def:Base

class Derived extends Base {}
//    ^^^^^^^ def:Derived
```

```query type_hierarchy def:Derived supertypes
main.ds:3:1-3:14 name=Base kind=class selection=main.ds:3:7-3:11
```

### Keep subtypes after malformed call statements

Type hierarchy should still resolve later subtypes after one malformed call statement.

```ds
broken(,

class Base {}
//    ^^^^ def:Base

class Derived extends Base {}
```

```query type_hierarchy def:Base subtypes
main.ds:5:1-5:30 name=Derived kind=class selection=main.ds:5:7-5:14
```
