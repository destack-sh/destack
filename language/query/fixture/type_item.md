
## Classes

### Return the same class item from its declaration and heritage uses

A class declaration and references in heritage clauses identify the same hierarchy item.

```tspp main.tspp
class Base {}
^^^^^^^^^^^^^ base_declaration
      ^^^^ base_name

class Derived extends Base {}
      ^^^^^^^ derived
                      ^^^^ base_use
```

```query type_item main.tspp#derived
@type_item.item name=Derived kind=class location=main.tspp:3:1-3:30 selection=main.tspp#derived symbol=main.tspp#Derived@2
```

```query type_item main.tspp#base_name
@type_item.item name=Base kind=class location=main.tspp#base_declaration selection=main.tspp#base_name symbol=main.tspp#Base@1
```

```query type_item main.tspp#base_use
@type_item.item name=Base kind=class location=main.tspp#base_declaration selection=main.tspp#base_name symbol=main.tspp#Base@1
```

### Resolve the current heritage target

A heritage reference identifies the current nominal declaration.

```tspp main.tspp
class First {}
^^^^^^^^^^^^^^ declaration:first
      ^^^^^ name:first
class Second {}
^^^^^^^^^^^^^^^ declaration:second
      ^^^^^^ name:second

class Derived extends First {}
                      ^^^^^ reference
```

```query type_item main.tspp#reference
@type_item.item name=First kind=class location=main.tspp#declaration:first selection=main.tspp#name:first symbol=main.tspp#First@1
```

```tspp main.tspp change
class First {}
^^^^^^^^^^^^^^ declaration:first
      ^^^^^ name:first
class Second {}
^^^^^^^^^^^^^^^ declaration:second
      ^^^^^^ name:second

class Derived extends Second {}
                      ^^^^^^ reference
```

```query type_item main.tspp#reference
@type_item.item name=Second kind=class location=main.tspp#declaration:second selection=main.tspp#name:second symbol=main.tspp#Second@2
```

## Structs

### Return a struct item

A struct declaration identifies its nominal hierarchy item.

```tspp main.tspp
struct Packet {}
^^^^^^^^^^^^^^^^ declaration
       ^^^^^^ name
```

```query type_item main.tspp#name
@type_item.item name=Packet kind=struct location=main.tspp#declaration selection=main.tspp#name symbol=main.tspp#Packet@1
```

## Enums

### Return an enum item

An enum declaration identifies its nominal hierarchy item.

```tspp main.tspp
enum Color { Red }
^^^^^^^^^^^^^^^^^^ declaration
     ^^^^^ name
```

```query type_item main.tspp#name
@type_item.item name=Color kind=enum location=main.tspp#declaration selection=main.tspp#name symbol=main.tspp#Color@1
```

## Interfaces

### Return an interface item

An interface position identifies its type hierarchy item.

```tspp main.tspp
interface Drawable {}
          ^^^^^^^^ drawable
```

```query type_item main.tspp#drawable
@type_item.item name=Drawable kind=interface location=main.tspp:1:1-1:22 selection=main.tspp#drawable symbol=main.tspp#Drawable@1
```

### Return a nominal interface item

A newtype interface uses its distinct nominal interface kind.

```tspp main.tspp
newtype interface Display {}
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ declaration
                  ^^^^^^^ name
```

```query type_item main.tspp#name
@type_item.item name=Display kind=newtype_interface location=main.tspp#declaration selection=main.tspp#name symbol=main.tspp#Display@1
```

## Newtypes

### Return a newtype item

A newtype identifies its nominal hierarchy item.

```tspp main.tspp
newtype UserId = string;
^^^^^^^^^^^^^^^^^^^^^^^ declaration
        ^^^^^^ name
```

```query type_item main.tspp#name
@type_item.item name=UserId kind=newtype location=main.tspp#declaration selection=main.tspp#name symbol=main.tspp#UserId@1
```

## Generics

### Include generic parameters

Generic parameters distinguish the declaration without changing its symbol identity.

```tspp main.tspp
struct Box<Value> {}
^^^^^^^^^^^^^^^^^^^^ declaration
       ^^^ name
```

```query type_item main.tspp#name
@type_item.item name=Box kind=struct generics="<Value>" location=main.tspp#declaration selection=main.tspp#name symbol=main.tspp#Box@1
```

### Return the generic declaration from an applied type

An applied type argument does not replace the nominal hierarchy item.

```tspp main.tspp
struct Box<Value> {}
^^^^^^^^^^^^^^^^^^^^ declaration
       ^^^ name

declare const value: Box<string>;
                     ^^^ type_use
```

```query type_item main.tspp#type_use
@type_item.item name=Box kind=struct generics="<Value>" location=main.tspp#declaration selection=main.tspp#name symbol=main.tspp#Box@1
```

## Imports

### Return an imported type item

An imported type position identifies the nominal type in its defining module.

```tspp library.tspp
export class Model {}
             ^^^^^ model
```

```tspp main.tspp
import { Model } from "./library.tspp";

declare const value: Model;
                     ^^^^^ imported_model
```

```query type_item main.tspp#imported_model
@type_item.item name=Model kind=class location=library.tspp:1:1-1:22 selection=library.tspp#model symbol=library.tspp#Model@1
```

### Follow aliased imports and re-exports

Plain aliases preserve the declaration's type symbol space through each module.

```tspp library.tspp
export struct Model {}
^^^^^^^^^^^^^^^^^^^^^^ declaration
              ^^^^^ name
```

```tspp public.tspp
export { Model as PublicModel } from "./library.tspp";
```

```tspp main.tspp
import { PublicModel as LocalModel } from "./public.tspp";
         ^^^^^^^^^^^ imported_name
                        ^^^^^^^^^^ local_name

declare const value: LocalModel;
                     ^^^^^^^^^^ type_use
```

```query type_item main.tspp#imported_name
@type_item.item name=Model kind=struct location=library.tspp#declaration selection=library.tspp#name symbol=library.tspp#Model@1
```

```query type_item main.tspp#local_name
@type_item.item name=Model kind=struct location=library.tspp#declaration selection=library.tspp#name symbol=library.tspp#Model@1
```

```query type_item main.tspp#type_use
@type_item.item name=Model kind=struct location=library.tspp#declaration selection=library.tspp#name symbol=library.tspp#Model@1
```

## Empty Results

### Return no item for a transparent type alias

A transparent alias does not introduce a nominal hierarchy identity.

```tspp main.tspp
type Identifier = string;
     ^^^^^^^^^^ identifier
```

```query type_item main.tspp#identifier
@type_item.none
```

### Return no item for a function

A function is not a type hierarchy item.

```tspp main.tspp
function render(): void {}
         ^^^^^^ render
```

```query type_item main.tspp#render
@type_item.none
```
