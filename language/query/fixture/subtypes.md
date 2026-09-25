
## Classes

### Find direct subclasses

Direct subclasses remain in declaration order.

```tspp main.tspp
class Base {}
      ^^^^ base

class First extends Base {}
      ^^^^^ first
class Second extends Base {}
      ^^^^^^ second
```

```query subtypes main.tspp#base
@subtypes.item name=First kind=class location=main.tspp:3:1-3:28 selection=main.tspp#first symbol=main.tspp#First@2
@subtypes.item name=Second kind=class location=main.tspp:4:1-4:29 selection=main.tspp#second symbol=main.tspp#Second@3
```

### Return only direct subclasses

Hierarchy expansion does not flatten descendants from later levels.

```tspp main.tspp
class Root {}
      ^^^^ root
class Middle extends Root {}
      ^^^^^^ middle
class Leaf extends Middle {}
      ^^^^ leaf
```

```query subtypes main.tspp#root
@subtypes.item name=Middle kind=class location=main.tspp:2:1-2:29 selection=main.tspp#middle symbol=main.tspp#Middle@2
```

### Return current direct subtypes

Subtype lookup includes declarations added by later edits.

```tspp main.tspp
class Base {}
      ^^^^ base

class First extends Base {}
^^^^^^^^^^^^^^^^^^^^^^^^^^^ declaration:first
      ^^^^^ first
```

```query subtypes main.tspp#base
@subtypes.item name=First kind=class location=main.tspp#declaration:first selection=main.tspp#first symbol=main.tspp#First@2
```

```tspp main.tspp change
class Base {}
      ^^^^ base

class First extends Base {}
^^^^^^^^^^^^^^^^^^^^^^^^^^^ declaration:first
      ^^^^^ first
class Second extends Base {}
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ declaration:second
      ^^^^^^ second
```

```query subtypes main.tspp#base
@subtypes.item name=First kind=class location=main.tspp#declaration:first selection=main.tspp#first symbol=main.tspp#First@2
@subtypes.item name=Second kind=class location=main.tspp#declaration:second selection=main.tspp#second symbol=main.tspp#Second@3
```

## Interfaces

### Find implementing classes and extending interfaces

Direct interface subtypes follow declaration order.

```tspp main.tspp
interface Base {}
          ^^^^ base

interface Derived extends Base {}
          ^^^^^^^ derived

class Implementation implements Base {}
      ^^^^^^^^^^^^^^ implementation
```

```query subtypes main.tspp#base
@subtypes.item name=Derived kind=interface location=main.tspp:3:1-3:34 selection=main.tspp#derived symbol=main.tspp#Derived@2
@subtypes.item name=Implementation kind=class location=main.tspp:5:1-5:40 selection=main.tspp#implementation symbol=main.tspp#Implementation@3
```

### Find nominal interface children

Nominal interfaces return their direct extension edges.

```tspp main.tspp
newtype interface PartialEqual {}
                  ^^^^^^^^^^^^ partial_equal

newtype interface Equal extends PartialEqual {}
                  ^^^^^ equal
```

```query subtypes main.tspp#partial_equal
@subtypes.item name=Equal kind=newtype_interface location=main.tspp:3:1-3:48 selection=main.tspp#equal symbol=main.tspp#Equal@2
```

## Implementations

### Find struct and enum implementations

Every implementation kind appears as a direct subtype.

```tspp main.tspp
interface Display {}
          ^^^^^^^ display

struct Packet implements Display {}
       ^^^^^^ packet

enum Status implements Display { Ready }
     ^^^^^^ status
```

```query subtypes main.tspp#display
@subtypes.item name=Packet kind=struct location=main.tspp:3:1-3:36 selection=main.tspp#packet symbol=main.tspp#Packet@2
@subtypes.item name=Status kind=enum location=main.tspp:5:1-5:41 selection=main.tspp#status symbol=main.tspp#Status@3
```

### Find implementations contributed by extensions

An extension contributes its nominal target as the implementing subtype.

```tspp main.tspp
newtype interface Display {}
                  ^^^^^^^ display

newtype UserId = string;
        ^^^^^^ user_id

extension of UserId implements Display {}
```

```query subtypes main.tspp#display
@subtypes.item name=UserId kind=newtype location=main.tspp:3:1-3:24 selection=main.tspp#user_id symbol=main.tspp#UserId@2
```

## Generics

### Include generic parameters

Generic subtype items display the parameters of their declarations.

```tspp main.tspp
newtype interface Container<Value> {}
                  ^^^^^^^^^ container

struct Box<Value> implements Container<Value> {}
       ^^^ box
```

```query subtypes main.tspp#container
@subtypes.item name=Box kind=struct generics="<Value>" location=main.tspp:3:1-3:49 selection=main.tspp#box symbol=main.tspp#Box@3
```

## Modules

### Find a subtype in another module

Program hierarchy indexes include cross-module inheritance edges.

```tspp base.tspp
export class Base {}
             ^^^^ base
```

```tspp implementation.tspp
import { Base } from "./base.tspp";

export class Derived extends Base {}
             ^^^^^^^ derived
```

```query subtypes base.tspp#base
@subtypes.item name=Derived kind=class location=implementation.tspp:3:1-3:37 selection=implementation.tspp#derived symbol=implementation.tspp#Derived@2
```

## Re-Exports

### Find a subtype through re-exports

Subtype lookup follows plain re-exports without losing the declaration's type symbol space.

```tspp types.tspp
export interface Renderable {}
                 ^^^^^^^^^^ renderable
```

```tspp barrel_a.tspp
export { Renderable as Surface } from "./types.tspp";
```

```tspp barrel_b.tspp
export { Surface } from "./barrel_a.tspp";
```

```tspp implementation.tspp
import { Surface } from "./barrel_b.tspp";

export class Sprite implements Surface {}
             ^^^^^^ sprite
```

```query subtypes types.tspp#renderable
@subtypes.item name=Sprite kind=class location=implementation.tspp:3:1-3:42 selection=implementation.tspp#sprite symbol=implementation.tspp#Sprite@2
```

## Empty Results

### Return no subtypes for a leaf class

A leaf class has no nominal children.

```tspp main.tspp
class Root {}

class Leaf extends Root {}
      ^^^^ leaf
```

```query subtypes main.tspp#leaf
@subtypes.none
```
