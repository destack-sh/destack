
## Classes

### Find a direct base class

The class reports its direct base class.

```tspp main.tspp
class Base {}
      ^^^^ base

class Derived extends Base {}
      ^^^^^^^ derived
```

```query supertypes main.tspp#derived
@supertypes.item name=Base kind=class location=main.tspp:1:1-1:14 selection=main.tspp#base symbol=main.tspp#Base@1
```

### Return class and interface parents in heritage order

The extended class precedes interfaces in the order written on the derived declaration.

```tspp main.tspp
class Base {}
      ^^^^ base
interface Writable {}
          ^^^^^^^^ writable
interface Readable {}
          ^^^^^^^^ readable

class Document extends Base implements Readable, Writable {}
      ^^^^^^^^ document
```

```query supertypes main.tspp#document
@supertypes.item name=Base kind=class location=main.tspp:1:1-1:14 selection=main.tspp#base symbol=main.tspp#Base@1
@supertypes.item name=Readable kind=interface location=main.tspp:3:1-3:22 selection=main.tspp#readable symbol=main.tspp#Readable@3
@supertypes.item name=Writable kind=interface location=main.tspp:2:1-2:22 selection=main.tspp#writable symbol=main.tspp#Writable@2
```

### Return only direct parents

Hierarchy expansion advances one declared edge at a time.

```tspp main.tspp
class Root {}
class Middle extends Root {}
      ^^^^^^ middle
class Leaf extends Middle {}
      ^^^^ leaf
```

```query supertypes main.tspp#leaf
@supertypes.item name=Middle kind=class location=main.tspp:2:1-2:29 selection=main.tspp#middle symbol=main.tspp#Middle@2
```

### Return the current direct supertype

Supertype lookup follows the current heritage declaration.

```tspp main.tspp
class First {}
^^^^^^^^^^^^^^ declaration:first
      ^^^^^ first
class Second {}
^^^^^^^^^^^^^^^ declaration:second
      ^^^^^^ second

class Derived extends First {}
      ^^^^^^^ derived
```

```query supertypes main.tspp#derived
@supertypes.item name=First kind=class location=main.tspp#declaration:first selection=main.tspp#first symbol=main.tspp#First@1
```

```tspp main.tspp change
class First {}
^^^^^^^^^^^^^^ declaration:first
      ^^^^^ first
class Second {}
^^^^^^^^^^^^^^^ declaration:second
      ^^^^^^ second

class Derived extends Second {}
      ^^^^^^^ derived
```

```query supertypes main.tspp#derived
@supertypes.item name=Second kind=class location=main.tspp#declaration:second selection=main.tspp#second symbol=main.tspp#Second@2
```

## Interfaces

### Find an implemented interface

An implementing class returns its direct interface edge.

```tspp main.tspp
interface Animal {}
          ^^^^^^ animal

class Dog implements Animal {}
      ^^^ dog
```

```query supertypes main.tspp#dog
@supertypes.item name=Animal kind=interface location=main.tspp:1:1-1:20 selection=main.tspp#animal symbol=main.tspp#Animal@1
```

### Find an extended interface

An extending interface returns its direct interface edge.

```tspp main.tspp
interface Base {}
          ^^^^ base

interface Derived extends Base {}
          ^^^^^^^ derived
```

```query supertypes main.tspp#derived
@supertypes.item name=Base kind=interface location=main.tspp:1:1-1:18 selection=main.tspp#base symbol=main.tspp#Base@1
```

### Return multiple extended interfaces in declaration order

An interface returns each direct parent in declaration order.

```tspp main.tspp
interface Readable {}
          ^^^^^^^^ readable
interface Writable {}
          ^^^^^^^^ writable

interface Document extends Readable, Writable {}
          ^^^^^^^^ document
```

```query supertypes main.tspp#document
@supertypes.item name=Readable kind=interface location=main.tspp:1:1-1:22 selection=main.tspp#readable symbol=main.tspp#Readable@1
@supertypes.item name=Writable kind=interface location=main.tspp:2:1-2:22 selection=main.tspp#writable symbol=main.tspp#Writable@2
```

### Find the parent of a nominal interface

Nominal interfaces participate in the same interface hierarchy.

```tspp main.tspp
newtype interface PartialEqual {}
                  ^^^^^^^^^^^^ partial_equal

newtype interface Equal extends PartialEqual {}
                  ^^^^^ equal
```

```query supertypes main.tspp#equal
@supertypes.item name=PartialEqual kind=newtype_interface location=main.tspp:1:1-1:34 selection=main.tspp#partial_equal symbol=main.tspp#PartialEqual@1
```

## Implementations

### Find interfaces implemented by structs and enums

Structs and enums return their direct implementation edges.

```tspp main.tspp
interface Display {}
          ^^^^^^^ display

struct Packet implements Display {}
       ^^^^^^ packet

enum Status implements Display { Ready }
     ^^^^^^ status
```

```query supertypes main.tspp#packet
@supertypes.item name=Display kind=interface location=main.tspp:1:1-1:21 selection=main.tspp#display symbol=main.tspp#Display@1
```

```query supertypes main.tspp#status
@supertypes.item name=Display kind=interface location=main.tspp:1:1-1:21 selection=main.tspp#display symbol=main.tspp#Display@1
```

### Find an implementation contributed by an extension

An extension implementation belongs to its nominal target.

```tspp main.tspp
newtype UserId = string;
        ^^^^^^ user_id

newtype interface Display {}
                  ^^^^^^^ display

extension of UserId implements Display {}
```

```query supertypes main.tspp#user_id
@supertypes.item name=Display kind=newtype_interface location=main.tspp:3:1-3:29 selection=main.tspp#display symbol=main.tspp#Display@2
```

## Generics

### Include generic parameters

Hierarchy items display the generic parameters of their declarations.

```tspp main.tspp
newtype interface Container<Value> {}
                  ^^^^^^^^^ name

struct Box<Value> implements Container<Value> {}
       ^^^ box
```

```query supertypes main.tspp#box
@supertypes.item name=Container kind=newtype_interface generics="<Value>" location=main.tspp:1:1-1:38 selection=main.tspp#name symbol=main.tspp#Container@1
```

## Modules

### Find a supertype through re-exports

Supertype lookup follows plain re-exports without losing the declaration's type symbol space.

```tspp types.tspp
export interface Renderable {}
                 ^^^^^^^^^^ renderable
```

```tspp public.tspp
export { Renderable as Surface } from "./types.tspp";
```

```tspp implementation.tspp
import { Surface } from "./public.tspp";

export class Sprite implements Surface {}
             ^^^^^^ sprite
```

```query supertypes implementation.tspp#sprite
@supertypes.item name=Renderable kind=interface location=types.tspp:1:1-1:31 selection=types.tspp#renderable symbol=types.tspp#Renderable@1
```

## Empty Results

### Return no supertypes for a root class

A root class has no nominal parent.

```tspp main.tspp
class Root {}
      ^^^^ root
```

```query supertypes main.tspp#root
@supertypes.none
```
