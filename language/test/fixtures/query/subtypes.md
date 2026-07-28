# Subtypes

## Classes

### Find direct subclasses

Direct subclasses remain in declaration order.

```ds main.ds
class Base {}
      ^^^^ base

class First extends Base {}
      ^^^^^ first
class Second extends Base {}
      ^^^^^^ second
```

```query subtypes main.ds#base
@subtypes.item name=First kind=class location=main.ds:3:1-3:28 selection=main.ds#first symbol=main.ds#First@2
@subtypes.item name=Second kind=class location=main.ds:4:1-4:29 selection=main.ds#second symbol=main.ds#Second@3
```

### Return only direct subclasses

Hierarchy expansion does not flatten descendants from later levels.

```ds main.ds
class Root {}
      ^^^^ root
class Middle extends Root {}
      ^^^^^^ middle
class Leaf extends Middle {}
      ^^^^ leaf
```

```query subtypes main.ds#root
@subtypes.item name=Middle kind=class location=main.ds:2:1-2:29 selection=main.ds#middle symbol=main.ds#Middle@2
```

## Interfaces

### Find implementing classes and extending interfaces

Direct interface subtypes follow declaration order.

```ds main.ds
interface Base {}
          ^^^^ base

interface Derived extends Base {}
          ^^^^^^^ derived

class Implementation implements Base {}
      ^^^^^^^^^^^^^^ implementation
```

```query subtypes main.ds#base
@subtypes.item name=Derived kind=interface location=main.ds:3:1-3:34 selection=main.ds#derived symbol=main.ds#Derived@2
@subtypes.item name=Implementation kind=class location=main.ds:5:1-5:40 selection=main.ds#implementation symbol=main.ds#Implementation@3
```

### Find nominal interface children

Nominal interfaces return their direct extension edges.

```ds main.ds
newtype interface PartialEqual {}
                  ^^^^^^^^^^^^ partial_equal

newtype interface Equal extends PartialEqual {}
                  ^^^^^ equal
```

```query subtypes main.ds#partial_equal
@subtypes.item name=Equal kind=newtype_interface location=main.ds:3:1-3:48 selection=main.ds#equal symbol=main.ds#Equal@2
```

## Implementations

### Find struct and enum implementations

Every implementation kind appears as a direct subtype.

```ds main.ds
interface Display {}
          ^^^^^^^ display

struct Packet implements Display {}
       ^^^^^^ packet

enum Status implements Display { Ready }
     ^^^^^^ status
```

```query subtypes main.ds#display
@subtypes.item name=Packet kind=struct location=main.ds:3:1-3:36 selection=main.ds#packet symbol=main.ds#Packet@2
@subtypes.item name=Status kind=enum location=main.ds:5:1-5:41 selection=main.ds#status symbol=main.ds#Status@3
```

### Find implementations contributed by extensions

An extension contributes its nominal target as the implementing subtype.

```ds main.ds
newtype interface Display {}
                  ^^^^^^^ display

newtype UserId = string;
        ^^^^^^ user_id

extension of UserId implements Display {}
```

```query subtypes main.ds#display
@subtypes.item name=UserId kind=newtype location=main.ds:3:1-3:24 selection=main.ds#user_id symbol=main.ds#UserId@2
```

## Generics

### Include generic declaration details

Generic subtype items display the parameters of their declarations.

```ds main.ds
newtype interface Container<Value> {}
                  ^^^^^^^^^ container

struct Box<Value> implements Container<Value> {}
       ^^^ box
```

```query subtypes main.ds#container
@subtypes.item name=Box kind=struct detail="<Value>" location=main.ds:3:1-3:49 selection=main.ds#box symbol=main.ds#Box@3
```

## Modules

### Find a subtype in another module

Program hierarchy indexes include cross-module inheritance edges.

```ds base.ds
export class Base {}
             ^^^^ base
```

```ds implementation.ds
import { Base } from "./base.ds";

export class Derived extends Base {}
             ^^^^^^^ derived
```

```query subtypes base.ds#base
@subtypes.item name=Derived kind=class location=implementation.ds:3:1-3:37 selection=implementation.ds#derived symbol=implementation.ds#Derived@2
```

## Re-Exports

### Find a subtype through re-exports

Subtype lookup follows plain re-exports without losing the declaration's type symbol space.

```ds types.ds
export interface Renderable {}
                 ^^^^^^^^^^ renderable
```

```ds barrel_a.ds
export { Renderable as Surface } from "./types.ds";
```

```ds barrel_b.ds
export { Surface } from "./barrel_a.ds";
```

```ds implementation.ds
import { Surface } from "./barrel_b.ds";

export class Sprite implements Surface {}
             ^^^^^^ sprite
```

```query subtypes types.ds#renderable
@subtypes.item name=Sprite kind=class location=implementation.ds:3:1-3:42 selection=implementation.ds#sprite symbol=implementation.ds#Sprite@2
```

## Empty Results

### Return no subtypes for a leaf class

A leaf class has no nominal children.

```ds main.ds
class Root {}

class Leaf extends Root {}
      ^^^^ leaf
```

```query subtypes main.ds#leaf
@subtypes.none
```
