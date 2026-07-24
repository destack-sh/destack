# Supertypes

## Classes

### Find a direct base class

The class reports its direct base class.

```ds main.ds
class Base {}
      ^^^^ base

class Derived extends Base {}
      ^^^^^^^ derived
```

```query supertypes main.ds#derived
@supertypes.item name=Base kind=class location=main.ds:1:1-1:14 selection=main.ds#base symbol=main.ds#Base@1
```

### Return class and interface parents in declaration order

The extended class precedes every implemented interface.

```ds main.ds
class Base {}
      ^^^^ base
interface Readable {}
          ^^^^^^^^ readable
interface Writable {}
          ^^^^^^^^ writable

class Document extends Base implements Readable, Writable {}
      ^^^^^^^^ document
```

```query supertypes main.ds#document
@supertypes.item name=Base kind=class location=main.ds:1:1-1:14 selection=main.ds#base symbol=main.ds#Base@1
@supertypes.item name=Readable kind=interface location=main.ds:2:1-2:22 selection=main.ds#readable symbol=main.ds#Readable@2
@supertypes.item name=Writable kind=interface location=main.ds:3:1-3:22 selection=main.ds#writable symbol=main.ds#Writable@3
```

### Return only direct parents

Hierarchy expansion advances one authored edge at a time.

```ds main.ds
class Root {}
class Middle extends Root {}
      ^^^^^^ middle
class Leaf extends Middle {}
      ^^^^ leaf
```

```query supertypes main.ds#leaf
@supertypes.item name=Middle kind=class location=main.ds:2:1-2:29 selection=main.ds#middle symbol=main.ds#Middle@2
```

## Interfaces

### Find an implemented interface

An implementing class retains its direct interface edge.

```ds main.ds
interface Animal {}
          ^^^^^^ animal

class Dog implements Animal {}
      ^^^ dog
```

```query supertypes main.ds#dog
@supertypes.item name=Animal kind=interface location=main.ds:1:1-1:20 selection=main.ds#animal symbol=main.ds#Animal@1
```

### Find an extended interface

An extending interface retains its direct interface edge.

```ds main.ds
interface Base {}
          ^^^^ base

interface Derived extends Base {}
          ^^^^^^^ derived
```

```query supertypes main.ds#derived
@supertypes.item name=Base kind=interface location=main.ds:1:1-1:18 selection=main.ds#base symbol=main.ds#Base@1
```

### Return multiple extended interfaces in declaration order

An interface retains each direct parent in the order it was declared.

```ds main.ds
interface Readable {}
          ^^^^^^^^ readable
interface Writable {}
          ^^^^^^^^ writable

interface Document extends Readable, Writable {}
          ^^^^^^^^ document
```

```query supertypes main.ds#document
@supertypes.item name=Readable kind=interface location=main.ds:1:1-1:22 selection=main.ds#readable symbol=main.ds#Readable@1
@supertypes.item name=Writable kind=interface location=main.ds:2:1-2:22 selection=main.ds#writable symbol=main.ds#Writable@2
```

### Find the parent of a nominal interface

Nominal interfaces participate in the same authored interface hierarchy.

```ds main.ds
newtype interface PartialEqual {}
                  ^^^^^^^^^^^^ partial_equal

newtype interface Equal extends PartialEqual {}
                  ^^^^^ equal
```

```query supertypes main.ds#equal
@supertypes.item name=PartialEqual kind=newtype_interface location=main.ds:1:1-1:34 selection=main.ds#partial_equal symbol=main.ds#PartialEqual@1
```

## Implementations

### Find interfaces implemented by structs and enums

Structs and enums retain their direct authored implementation edges.

```ds main.ds
interface Display {}
          ^^^^^^^ display

struct Packet implements Display {}
       ^^^^^^ packet

enum Status implements Display { Ready }
     ^^^^^^ status
```

```query supertypes main.ds#packet
@supertypes.item name=Display kind=interface location=main.ds:1:1-1:21 selection=main.ds#display symbol=main.ds#Display@1
```

```query supertypes main.ds#status
@supertypes.item name=Display kind=interface location=main.ds:1:1-1:21 selection=main.ds#display symbol=main.ds#Display@1
```

### Find an implementation contributed by an extension

An extension implementation belongs to its nominal target.

```ds main.ds
newtype UserId = string;
        ^^^^^^ user_id

newtype interface Display {}
                  ^^^^^^^ display

extension of UserId implements Display {}
```

```query supertypes main.ds#user_id
@supertypes.item name=Display kind=newtype_interface location=main.ds:3:1-3:29 selection=main.ds#display symbol=main.ds#Display@2
```

## Generics

### Retain generic declaration details

Hierarchy items display the generic parameters of their declarations.

```ds main.ds
newtype interface Container<Value> {}
                  ^^^^^^^^^ name

struct Box<Value> implements Container<Value> {}
       ^^^ box
```

```query supertypes main.ds#box
@supertypes.item name=Container kind=newtype_interface detail="<Value>" location=main.ds:1:1-1:38 selection=main.ds#name symbol=main.ds#Container@1
```

## Modules

### Find a supertype through re-exports

Supertype lookup follows plain re-exports without losing the declaration's type symbol space.

```ds types.ds
export interface Renderable {}
                 ^^^^^^^^^^ renderable
```

```ds public.ds
export { Renderable as Surface } from "./types.ds";
```

```ds implementation.ds
import { Surface } from "./public.ds";

export class Sprite implements Surface {}
             ^^^^^^ sprite
```

```query supertypes implementation.ds#sprite
@supertypes.item name=Renderable kind=interface location=types.ds:1:1-1:31 selection=types.ds#renderable symbol=types.ds#Renderable@1
```

## Empty Results

### Return no supertypes for a root class

A root class has no nominal parent.

```ds main.ds
class Root {}
      ^^^^ root
```

```query supertypes main.ds#root
@supertypes.none
```
