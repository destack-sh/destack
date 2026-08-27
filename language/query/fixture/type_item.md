
## Classes

### Return the same class item from its declaration and heritage uses

A class declaration and references in heritage clauses identify the same hierarchy item.

```ds main.ds
class Base {}
^^^^^^^^^^^^^ base_declaration
      ^^^^ base_name

class Derived extends Base {}
      ^^^^^^^ derived
                      ^^^^ base_use
```

```query type_item main.ds#derived
@type_item.item name=Derived kind=class location=main.ds:3:1-3:30 selection=main.ds#derived symbol=main.ds#Derived@2
```

```query type_item main.ds#base_name
@type_item.item name=Base kind=class location=main.ds#base_declaration selection=main.ds#base_name symbol=main.ds#Base@1
```

```query type_item main.ds#base_use
@type_item.item name=Base kind=class location=main.ds#base_declaration selection=main.ds#base_name symbol=main.ds#Base@1
```

### Resolve the current heritage target

A heritage reference identifies the current nominal declaration.

```ds main.ds
class First {}
^^^^^^^^^^^^^^ declaration:first
      ^^^^^ name:first
class Second {}
^^^^^^^^^^^^^^^ declaration:second
      ^^^^^^ name:second

class Derived extends First {}
                      ^^^^^ reference
```

```query type_item main.ds#reference
@type_item.item name=First kind=class location=main.ds#declaration:first selection=main.ds#name:first symbol=main.ds#First@1
```

```ds main.ds change
class First {}
^^^^^^^^^^^^^^ declaration:first
      ^^^^^ name:first
class Second {}
^^^^^^^^^^^^^^^ declaration:second
      ^^^^^^ name:second

class Derived extends Second {}
                      ^^^^^^ reference
```

```query type_item main.ds#reference
@type_item.item name=Second kind=class location=main.ds#declaration:second selection=main.ds#name:second symbol=main.ds#Second@2
```

## Structs

### Return a struct item

A struct declaration identifies its nominal hierarchy item.

```ds main.ds
struct Packet {}
^^^^^^^^^^^^^^^^ declaration
       ^^^^^^ name
```

```query type_item main.ds#name
@type_item.item name=Packet kind=struct location=main.ds#declaration selection=main.ds#name symbol=main.ds#Packet@1
```

## Enums

### Return an enum item

An enum declaration identifies its nominal hierarchy item.

```ds main.ds
enum Color { Red }
^^^^^^^^^^^^^^^^^^ declaration
     ^^^^^ name
```

```query type_item main.ds#name
@type_item.item name=Color kind=enum location=main.ds#declaration selection=main.ds#name symbol=main.ds#Color@1
```

## Interfaces

### Return an interface item

An interface position identifies its type hierarchy item.

```ds main.ds
interface Drawable {}
          ^^^^^^^^ drawable
```

```query type_item main.ds#drawable
@type_item.item name=Drawable kind=interface location=main.ds:1:1-1:22 selection=main.ds#drawable symbol=main.ds#Drawable@1
```

### Return a nominal interface item

A newtype interface uses its distinct nominal interface kind.

```ds main.ds
newtype interface Display {}
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ declaration
                  ^^^^^^^ name
```

```query type_item main.ds#name
@type_item.item name=Display kind=newtype_interface location=main.ds#declaration selection=main.ds#name symbol=main.ds#Display@1
```

## Newtypes

### Return a newtype item

A newtype identifies its nominal hierarchy item.

```ds main.ds
newtype UserId = string;
^^^^^^^^^^^^^^^^^^^^^^^ declaration
        ^^^^^^ name
```

```query type_item main.ds#name
@type_item.item name=UserId kind=newtype location=main.ds#declaration selection=main.ds#name symbol=main.ds#UserId@1
```

## Generics

### Include generic parameters

Generic parameters distinguish the declaration without changing its symbol identity.

```ds main.ds
struct Box<Value> {}
^^^^^^^^^^^^^^^^^^^^ declaration
       ^^^ name
```

```query type_item main.ds#name
@type_item.item name=Box kind=struct generics="<Value>" location=main.ds#declaration selection=main.ds#name symbol=main.ds#Box@1
```

### Return the generic declaration from an applied type

An applied type argument does not replace the nominal hierarchy item.

```ds main.ds
struct Box<Value> {}
^^^^^^^^^^^^^^^^^^^^ declaration
       ^^^ name

declare const value: Box<string>;
                     ^^^ type_use
```

```query type_item main.ds#type_use
@type_item.item name=Box kind=struct generics="<Value>" location=main.ds#declaration selection=main.ds#name symbol=main.ds#Box@1
```

## Imports

### Return an imported type item

An imported type position identifies the nominal type in its defining module.

```ds library.ds
export class Model {}
             ^^^^^ model
```

```ds main.ds
import { Model } from "./library.ds";

declare const value: Model;
                     ^^^^^ imported_model
```

```query type_item main.ds#imported_model
@type_item.item name=Model kind=class location=library.ds:1:1-1:22 selection=library.ds#model symbol=library.ds#Model@1
```

### Follow aliased imports and re-exports

Plain aliases preserve the declaration's type symbol space through each module.

```ds library.ds
export struct Model {}
^^^^^^^^^^^^^^^^^^^^^^ declaration
              ^^^^^ name
```

```ds public.ds
export { Model as PublicModel } from "./library.ds";
```

```ds main.ds
import { PublicModel as LocalModel } from "./public.ds";
         ^^^^^^^^^^^ imported_name
                        ^^^^^^^^^^ local_name

declare const value: LocalModel;
                     ^^^^^^^^^^ type_use
```

```query type_item main.ds#imported_name
@type_item.item name=Model kind=struct location=library.ds#declaration selection=library.ds#name symbol=library.ds#Model@1
```

```query type_item main.ds#local_name
@type_item.item name=Model kind=struct location=library.ds#declaration selection=library.ds#name symbol=library.ds#Model@1
```

```query type_item main.ds#type_use
@type_item.item name=Model kind=struct location=library.ds#declaration selection=library.ds#name symbol=library.ds#Model@1
```

## Empty Results

### Return no item for a transparent type alias

A transparent alias does not introduce a nominal hierarchy identity.

```ds main.ds
type Identifier = string;
     ^^^^^^^^^^ identifier
```

```query type_item main.ds#identifier
@type_item.none
```

### Return no item for a function

A function is not a type hierarchy item.

```ds main.ds
function render(): void {}
         ^^^^^^ render
```

```query type_item main.ds#render
@type_item.none
```
