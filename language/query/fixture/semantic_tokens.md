
## Symbols

### Highlight declarations and references

Declarations and references receive highlighting for their resolved declaration kinds.

```tspp main.tspp
function identity(value: int32): int32 {
         ^^^^^^^^ function
                  ^^^^^ parameter
    return value;
           ^^^^^ reference
}
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#function type=function modifiers=declaration
@semantic_tokens.token range=main.tspp#parameter type=parameter modifiers=declaration
@semantic_tokens.token range=main.tspp#reference type=parameter
```

### Preserve struct construction highlighting after a rename

Struct construction types remain highlighted after their declaration and reference are renamed.

```tspp main.tspp
struct Point {}
       ^^^^^ declaration

const point = Point {};
      ^^^^^ binding
              ^^^^^ reference
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#declaration type=struct modifiers=declaration
@semantic_tokens.token range=main.tspp#binding type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp#reference type=struct
```

```diff main.tspp
@@ -1,2 +1,2 @@
-struct Point {}
+struct Shape {}
        ^^^^^ declaration
@@ -4,3 +4,3 @@
-const point = Point {};
+const point = Shape {};
       ^^^^^ binding
               ^^^^^ reference
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#declaration type=struct modifiers=declaration
@semantic_tokens.token range=main.tspp#binding type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp#reference type=struct
```

### Preserve import highlighting after an alias rename

Imported declarations and local aliases remain distinct after the alias changes.

```tspp library.tspp
export function paint(): void {}
```

```tspp main.tspp
import { paint as render } from "./library.tspp";
         ^^^^^ imported
                  ^^^^^^ alias

render();
^^^^^^ reference
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#imported type=function
@semantic_tokens.token range=main.tspp#alias type=function modifiers=declaration
@semantic_tokens.token range=main.tspp#reference type=function
```

```diff main.tspp
@@ -1,3 +1,3 @@
-import { paint as render } from "./library.tspp";
+import { paint as finish } from "./library.tspp";
          ^^^^^ imported
-                  ^^^^^^ alias
+                  ^^^^^^ alias
@@ -5,2 +5,2 @@
-render();
-^^^^^^ reference
+finish();
+^^^^^^ reference
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#imported type=function
@semantic_tokens.token range=main.tspp#alias type=function modifiers=declaration
@semantic_tokens.token range=main.tspp#reference type=function
```

### Highlight a public re-export alias

Re-exported names use the target declaration kind without changing the public alias.

```tspp library.tspp
export function paint(): void {}
```

```tspp barrel.tspp
export { paint as render } from "./library.tspp";
         ^^^^^ imported
                  ^^^^^^ alias
```

```tspp main.tspp
import { render } from "./barrel.tspp";
         ^^^^^^ imported

render();
^^^^^^ reference
```

```query semantic_tokens barrel.tspp
@semantic_tokens.token range=barrel.tspp#imported type=function
@semantic_tokens.token range=barrel.tspp#alias type=function modifiers=declaration
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#imported type=function modifiers=declaration
@semantic_tokens.token range=main.tspp#reference type=function
```

### Preserve object pattern highlighting after a field edit

Object patterns distinguish the selected field from the introduced binding after both names change.

```tspp main.tspp
struct Box {
       ^^^ structure
    value: int32;
    ^^^^^ declaration
}

declare const box: Box;
              ^^^ box
                   ^^^ box_type
const { value: item } = box;
        ^^^^^ field
               ^^^^ binding
                        ^^^ box_reference
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#structure type=struct modifiers=declaration
@semantic_tokens.token range=main.tspp#declaration type=property modifiers=declaration
@semantic_tokens.token range=main.tspp#box type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp#box_type type=struct
@semantic_tokens.token range=main.tspp#field type=property
@semantic_tokens.token range=main.tspp#binding type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp#box_reference type=variable modifiers=readonly
```

```diff main.tspp
@@ -1,5 +1,5 @@
 struct Box {
        ^^^ structure
-    value: int32;
+    count: int32;
     ^^^^^ declaration
 }
@@ -7,7 +7,7 @@
 declare const box: Box;
               ^^^ box
                    ^^^ box_type
-const { value: item } = box;
+const { count: size } = box;
         ^^^^^ field
                ^^^^ binding
                         ^^^ box_reference
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#structure type=struct modifiers=declaration
@semantic_tokens.token range=main.tspp#declaration type=property modifiers=declaration
@semantic_tokens.token range=main.tspp#box type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp#box_type type=struct
@semantic_tokens.token range=main.tspp#field type=property
@semantic_tokens.token range=main.tspp#binding type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp#box_reference type=variable modifiers=readonly
```

### Preserve interface member highlighting after a signature edit

Interface methods and their parameters remain distinct when the signature changes.

```tspp main.tspp
interface Reader {
          ^^^^^^ interface
    read(value: string): string;
    ^^^^ method
         ^^^^^ parameter
}
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#interface type=interface modifiers=declaration
@semantic_tokens.token range=main.tspp#method type=method modifiers=declaration,abstract
@semantic_tokens.token range=main.tspp#parameter type=parameter modifiers=declaration
```

```diff main.tspp
@@ -1,6 +1,6 @@
 interface Reader {
           ^^^^^^ interface
-    read(value: string): string;
-    ^^^^ method
+    load(input: string): string;
+    ^^^^ method
          ^^^^^ parameter
 }
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#interface type=interface modifiers=declaration
@semantic_tokens.token range=main.tspp#method type=method modifiers=declaration,abstract
@semantic_tokens.token range=main.tspp#parameter type=parameter modifiers=declaration
```

### Preserve generic and enum highlighting after renames

Generic parameters and enum declarations remain highlighted when their names change.

```tspp main.tspp
extension<Element> of string {}
          ^^^^^^^ parameter

enum MemoryOrdering {}
     ^^^^^^^^^^^^^^ enumeration
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#parameter type=type_parameter modifiers=declaration
@semantic_tokens.token range=main.tspp#enumeration type=enum modifiers=declaration
```

```diff main.tspp
@@ -1,2 +1,2 @@
-extension<Element> of string {}
-          ^^^^^^^ parameter
+extension<Value> of string {}
+          ^^^^^ parameter
@@ -4,2 +4,2 @@
-enum MemoryOrdering {}
-     ^^^^^^^^^^^^^^ enumeration
+enum Ordering {}
+     ^^^^^^^^ enumeration
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#parameter type=type_parameter modifiers=declaration
@semantic_tokens.token range=main.tspp#enumeration type=enum modifiers=declaration
```

### Highlight nominal declarations and members

Nominal declarations and their members receive distinct highlighting.

```tspp main.tspp
struct Point {
       ^^^^^ point
    x: int32;
    ^ field
}

class Shape {}
      ^^^^^ shape

enum Color {
     ^^^^^ color
    Red,
    ^^^ red
}
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#point type=struct modifiers=declaration
@semantic_tokens.token range=main.tspp#field type=property modifiers=declaration
@semantic_tokens.token range=main.tspp#shape type=class modifiers=declaration
@semantic_tokens.token range=main.tspp#color type=enum modifiers=declaration
@semantic_tokens.token range=main.tspp#red type=enum_member modifiers=declaration,readonly
```

### Highlight forward struct references and fields

Forward references distinguish types, construction fields, and member reads.

```tspp main.tspp
class Player {
      ^^^^^^ player
    position: Position;
    ^^^^^^^^ player_position
              ^^^^^^^^ forward_position
}

struct Position {
       ^^^^^^^^ position
    x: float64;
    ^ x_declaration
    y: float64;
    ^ y_declaration
}

const position = Position { x: 1.0, y: 2.0 };
      ^^^^^^^^ binding
                 ^^^^^^^^ construction
                            ^ x_construction
                                    ^ y_construction
const x = position.x;
      ^ local
          ^^^^^^^^ receiver
                   ^ member
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#player type=class modifiers=declaration
@semantic_tokens.token range=main.tspp#player_position type=property modifiers=declaration
@semantic_tokens.token range=main.tspp#forward_position type=struct
@semantic_tokens.token range=main.tspp#position type=struct modifiers=declaration
@semantic_tokens.token range=main.tspp#x_declaration type=property modifiers=declaration
@semantic_tokens.token range=main.tspp#y_declaration type=property modifiers=declaration
@semantic_tokens.token range=main.tspp#binding type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp#construction type=struct
@semantic_tokens.token range=main.tspp#x_construction type=property
@semantic_tokens.token range=main.tspp#y_construction type=property
@semantic_tokens.token range=main.tspp#local type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp#receiver type=variable modifiers=readonly
@semantic_tokens.token range=main.tspp#member type=property
```

### Highlight object fields

Object fields distinguish declarations, contextual references, and shorthand values.

```tspp main.tspp
interface Options {
          ^^^^^^^ options
    enabled: boolean;
    ^^^^^^^ member
}

const enabled = true;
      ^^^^^^^ binding
const inferred = { value: enabled, enabled };
      ^^^^^^^^ inferred
                   ^^^^^ field
                          ^^^^^^^ value
                                   ^^^^^^^ shorthand
const contextual: Options = { enabled: true };
      ^^^^^^^^^^ contextual
                  ^^^^^^^ contextual_type
                              ^^^^^^^ contextual_field
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#options type=interface modifiers=declaration
@semantic_tokens.token range=main.tspp#member type=property modifiers=declaration
@semantic_tokens.token range=main.tspp#binding type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp#inferred type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp#field type=property modifiers=declaration
@semantic_tokens.token range=main.tspp#value type=variable modifiers=readonly
@semantic_tokens.token range=main.tspp#shorthand type=variable modifiers=readonly
@semantic_tokens.token range=main.tspp#contextual type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp#contextual_type type=interface
@semantic_tokens.token range=main.tspp#contextual_field type=property
```

### Highlight implicit abstract interface methods

An interface method is abstract even when it omits an explicit modifier.

```tspp main.tspp
interface Drawable {
          ^^^^^^^^ drawable
    draw(): void;
    ^^^^ method
}
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#drawable type=interface modifiers=declaration
@semantic_tokens.token range=main.tspp#method type=method modifiers=declaration,abstract
```

### Highlight implicit abstract associated types

An associated type without a definition is an abstract interface requirement.


```tspp main.tspp
interface Container {
          ^^^^^^^^^ container
    type Item;
         ^^^^ item
}
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#container type=interface modifiers=declaration
@semantic_tokens.token range=main.tspp#item type=type modifiers=declaration,abstract
```

### Highlight implicit abstract associated constants

An associated constant without a value is an abstract interface requirement.


```tspp main.tspp
interface Container {
          ^^^^^^^^^ container
    const Width: usize;
          ^^^^^ width
}
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#container type=interface modifiers=declaration
@semantic_tokens.token range=main.tspp#width type=property modifiers=declaration,readonly,abstract
```

### Highlight interface method signatures

Interface method parameters and their referenced types receive distinct highlighting.

```tspp main.tspp
struct ResourceId {}
       ^^^^^^^^^^ resource_type

struct IoControlRequest {}
       ^^^^^^^^^^^^^^^^ request_type

interface IoControlBinding {
          ^^^^^^^^^^^^^^^^ binding_type
    @binding("tspp.io.control", {
     ^^^^^^^ decorator
        provider: "host",
        ^^^^^^^^ provider
        effect: "external",
        ^^^^^^ effect
        requires: ["host.fs.metadata"],
        ^^^^^^^^ requires
        families: ["windows", "unix"],
        ^^^^^^^^ families
    })
    executeIoControl(
    ^^^^^^^^^^^^^^^^ method
        resource: ResourceId,
        ^^^^^^^^ resource_parameter
                  ^^^^^^^^^^ resource_reference
        request: IoControlRequest,
        ^^^^^^^ request_parameter
                 ^^^^^^^^^^^^^^^^ request_reference
    ): IoControlRequest;
       ^^^^^^^^^^^^^^^^ return_reference
}
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#resource_type type=struct modifiers=declaration
@semantic_tokens.token range=main.tspp#request_type type=struct modifiers=declaration
@semantic_tokens.token range=main.tspp#binding_type type=interface modifiers=declaration
@semantic_tokens.token range=main.tspp#decorator type=decorator modifiers=default_library
@semantic_tokens.token range=main.tspp#provider type=property modifiers=declaration
@semantic_tokens.token range=main.tspp#effect type=property modifiers=declaration
@semantic_tokens.token range=main.tspp#requires type=property modifiers=declaration
@semantic_tokens.token range=main.tspp#families type=property modifiers=declaration
@semantic_tokens.token range=main.tspp#method type=method modifiers=declaration,abstract
@semantic_tokens.token range=main.tspp#resource_parameter type=parameter modifiers=declaration
@semantic_tokens.token range=main.tspp#resource_reference type=struct
@semantic_tokens.token range=main.tspp#request_parameter type=parameter modifiers=declaration
@semantic_tokens.token range=main.tspp#request_reference type=struct
@semantic_tokens.token range=main.tspp#return_reference type=struct
```

### Highlight binding declarations

A binding module distinguishes imported, declared, and referenced symbols throughout its source.

```tspp main.tspp
import { HostError } from "./host.tspp";
         ^^^^^^^^^ host_error_import
import { ResourceId } from "./resource.tspp";
         ^^^^^^^^^^ resource_import

/// Resource control flags.
export newtype IoControlFlags = uint32;
               ^^^^^^^^^^^^^^ flags_type

/// Resource control request.
export struct IoControlRequest {
              ^^^^^^^^^^^^^^^^ request_type
    /// Host request code.
    code: uint64;
    ^^^^ code_field
    /// Scalar argument value.
    argument: uint64;
    ^^^^^^^^ argument_field
    /// Request flags.
    flags: IoControlFlags;
    ^^^^^ flags_field
           ^^^^^^^^^^^^^^ flags_reference
}

/// Resource control result.
export struct IoControlResult {
              ^^^^^^^^^^^^^^^ result_type
    /// Host return value.
    value: int64;
    ^^^^^ value_field
    /// Output bytes written.
    bytes: uint32;
    ^^^^^ bytes_field
}

/// Control binding family.
export interface IoControlBinding {
                 ^^^^^^^^^^^^^^^^ binding_type
    /// Execute one resource control request.
    @binding("tspp.io.control", {
     ^^^^^^^ decorator
        provider: "host",
        ^^^^^^^^ provider
        effect: "external",
        ^^^^^^ effect
        requires: ["host.fs.metadata"],
        ^^^^^^^^ requires
        families: ["windows", "unix"],
        ^^^^^^^^ families
    })
    executeIoControl(
    ^^^^^^^^^^^^^^^^ method
        resource: ResourceId,
        ^^^^^^^^ resource_parameter
                  ^^^^^^^^^^ resource_reference
        request: IoControlRequest,
        ^^^^^^^ request_parameter
                 ^^^^^^^^^^^^^^^^ request_reference
        input: &readonly [uint8],
        ^^^^^ input_parameter
        output: &[uint8],
        ^^^^^^ output_parameter
    ): Result<IoControlResult, HostError>;
       ^^^^^^ result_reference
              ^^^^^^^^^^^^^^^ result_type_reference
                               ^^^^^^^^^ host_error_reference
}
```

```tspp host.tspp
export interface HostError {}
```

```tspp resource.tspp
export struct ResourceId {}
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#host_error_import type=interface modifiers=declaration
@semantic_tokens.token range=main.tspp#resource_import type=struct modifiers=declaration
@semantic_tokens.token range=main.tspp:4:1-4:28 type=comment modifiers=documentation
@semantic_tokens.token range=main.tspp#flags_type type=type modifiers=declaration
@semantic_tokens.token range=main.tspp:7:1-7:30 type=comment modifiers=documentation
@semantic_tokens.token range=main.tspp#request_type type=struct modifiers=declaration
@semantic_tokens.token range=main.tspp:9:5-9:27 type=comment modifiers=documentation
@semantic_tokens.token range=main.tspp#code_field type=property modifiers=declaration
@semantic_tokens.token range=main.tspp:11:5-11:31 type=comment modifiers=documentation
@semantic_tokens.token range=main.tspp#argument_field type=property modifiers=declaration
@semantic_tokens.token range=main.tspp:13:5-13:23 type=comment modifiers=documentation
@semantic_tokens.token range=main.tspp#flags_field type=property modifiers=declaration
@semantic_tokens.token range=main.tspp#flags_reference type=type
@semantic_tokens.token range=main.tspp:17:1-17:29 type=comment modifiers=documentation
@semantic_tokens.token range=main.tspp#result_type type=struct modifiers=declaration
@semantic_tokens.token range=main.tspp:19:5-19:27 type=comment modifiers=documentation
@semantic_tokens.token range=main.tspp#value_field type=property modifiers=declaration
@semantic_tokens.token range=main.tspp:21:5-21:30 type=comment modifiers=documentation
@semantic_tokens.token range=main.tspp#bytes_field type=property modifiers=declaration
@semantic_tokens.token range=main.tspp:25:1-25:28 type=comment modifiers=documentation
@semantic_tokens.token range=main.tspp#binding_type type=interface modifiers=declaration
@semantic_tokens.token range=main.tspp:27:5-27:46 type=comment modifiers=documentation
@semantic_tokens.token range=main.tspp#decorator type=decorator modifiers=default_library
@semantic_tokens.token range=main.tspp#provider type=property modifiers=declaration
@semantic_tokens.token range=main.tspp#effect type=property modifiers=declaration
@semantic_tokens.token range=main.tspp#requires type=property modifiers=declaration
@semantic_tokens.token range=main.tspp#families type=property modifiers=declaration
@semantic_tokens.token range=main.tspp#method type=method modifiers=declaration,abstract
@semantic_tokens.token range=main.tspp#resource_parameter type=parameter modifiers=declaration
@semantic_tokens.token range=main.tspp#resource_reference type=struct
@semantic_tokens.token range=main.tspp#request_parameter type=parameter modifiers=declaration
@semantic_tokens.token range=main.tspp#request_reference type=struct
@semantic_tokens.token range=main.tspp#input_parameter type=parameter modifiers=declaration
@semantic_tokens.token range=main.tspp#output_parameter type=parameter modifiers=declaration
@semantic_tokens.token range=main.tspp#result_reference type=type modifiers=default_library
@semantic_tokens.token range=main.tspp#result_type_reference type=struct
@semantic_tokens.token range=main.tspp#host_error_reference type=interface
```

### Omit unnamed interface signatures

Call signatures do not introduce a semantic symbol.

```tspp main.tspp
interface Callable {
          ^^^^^^^^ callable
    (value: string): string;
    named(): void;
    ^^^^^ named
}
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#callable type=interface modifiers=declaration
@semantic_tokens.token range=main.tspp:2:6-2:11 type=parameter modifiers=declaration
@semantic_tokens.token range=main.tspp#named type=method modifiers=declaration,abstract
```

### Highlight receiver parameters

Receiver modifiers retain their ordinary highlighting while `this` is the declared parameter.

```tspp main.tspp
interface Borrow<T> {
          ^^^^^^ interface
                 ^ generic
    borrow(&readonly this): &readonly T;
    ^^^^^^ method
                     ^^^^ receiver
                                      ^ result
}
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#interface type=interface modifiers=declaration
@semantic_tokens.token range=main.tspp#generic type=type_parameter modifiers=declaration
@semantic_tokens.token range=main.tspp#method type=method modifiers=declaration,abstract
@semantic_tokens.token range=main.tspp#receiver type=parameter modifiers=declaration
@semantic_tokens.token range=main.tspp#result type=type_parameter
```

### Highlight nominal declarations

Type aliases, newtypes, nominal interfaces, and extensions receive distinct highlighting.

```tspp main.tspp
type Identifier = uint64;
     ^^^^^^^^^^ type_alias

newtype UserId = Identifier;
        ^^^^^^ newtype
                 ^^^^^^^^^^ newtype_value

newtype interface Display {}
                  ^^^^^^^ nominal_interface

struct Report {}
       ^^^^^^ struct

extension BufferAccess of UserId {}
          ^^^^^^^^^^^^ extension
                          ^^^^^^ extension_target

extension of UserId {}
             ^^^^^^ unnamed_extension_target

extension of UserId implements Display {}
             ^^^^^^ repeated_extension_target
                               ^^^^^^^ implemented_interface

extension of Report implements Display {}
             ^^^^^^ struct_extension_target
                               ^^^^^^^ implemented_interface_again
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#type_alias type=type modifiers=declaration
@semantic_tokens.token range=main.tspp#newtype type=type modifiers=declaration
@semantic_tokens.token range=main.tspp#newtype_value type=type
@semantic_tokens.token range=main.tspp#nominal_interface type=interface modifiers=declaration
@semantic_tokens.token range=main.tspp#struct type=struct modifiers=declaration
@semantic_tokens.token range=main.tspp#extension type=type modifiers=declaration
@semantic_tokens.token range=main.tspp#extension_target type=type
@semantic_tokens.token range=main.tspp#unnamed_extension_target type=type
@semantic_tokens.token range=main.tspp#repeated_extension_target type=type
@semantic_tokens.token range=main.tspp#implemented_interface type=interface
@semantic_tokens.token range=main.tspp#struct_extension_target type=struct
@semantic_tokens.token range=main.tspp#implemented_interface_again type=interface
```

### Highlight generic declarations and references

Generic type and const value parameters remain distinct from nominal types and locals.

```tspp main.tspp
function identity<Value, const size: usize>(value: Value): Value {
         ^^^^^^^^ function
                  ^^^^^ generic
                               ^^^^ size_declaration
                                            ^^^^^ parameter
                                                   ^^^^^ parameter_type
                                                           ^^^^^ return_type
    const buffer: [uint8; size] = [];
          ^^^^^^ buffer
                          ^^^^ size_reference
    return value;
           ^^^^^ value_reference
}
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#function type=function modifiers=declaration
@semantic_tokens.token range=main.tspp#generic type=type_parameter modifiers=declaration
@semantic_tokens.token range=main.tspp#size_declaration type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp#parameter type=parameter modifiers=declaration
@semantic_tokens.token range=main.tspp#parameter_type type=type_parameter
@semantic_tokens.token range=main.tspp#return_type type=type_parameter
@semantic_tokens.token range=main.tspp#buffer type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp#size_reference type=variable modifiers=readonly
@semantic_tokens.token range=main.tspp#value_reference type=parameter
```

### Highlight extension type parameters

Extension type parameters receive the same declaration highlighting as other generic type parameters.

```tspp main.tspp
extension<Element> of string {}
          ^^^^^^^ parameter
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#parameter type=type_parameter modifiers=declaration
```

### Highlight lifetime declarations and references

Lifetime declarations and references are readonly variables.

```tspp main.tspp
function borrow<'a>(value: &'a readonly string): &'a readonly string {
         ^^^^^^ function
                ^^ lifetime_declaration
                    ^^^^^ parameter
                            ^^ parameter_lifetime
                                                  ^^ return_lifetime
    return value;
           ^^^^^ value_reference
}
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#function type=function modifiers=declaration
@semantic_tokens.token range=main.tspp#lifetime_declaration type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp#parameter type=parameter modifiers=declaration
@semantic_tokens.token range=main.tspp#parameter_lifetime type=variable modifiers=readonly
@semantic_tokens.token range=main.tspp#return_lifetime type=variable modifiers=readonly
@semantic_tokens.token range=main.tspp#value_reference type=parameter
```

### Highlight type parameters, tuple labels, and associated refinements

Mapped keys, inferred parameters, index keys, tuple labels, nested generics, and associated
refinements receive their matching highlights.

```tspp main.tspp
type Transform<Source> = {
     ^^^^^^^^^ transform
               ^^^^^^ source_declaration
    [Key in keyof Source]: Source[Key];
     ^^^ mapped_declaration
                  ^^^^^^ mapped_source
                           ^^^^^^ mapped_value
                                  ^^^ mapped_key
};

type Element<Value> = Value extends (infer Item)[] ? Item : never;
     ^^^^^^^ element
             ^^^^^ value_declaration
                      ^^^^^ value_reference
                                           ^^^^ infer_declaration
                                                     ^^^^ infer_reference

type Pair = [first: int32, ...rest: string[]];
     ^^^^ pair
             ^^^^^ first
                              ^^^^ rest

interface Dictionary {
          ^^^^^^^^^^ dictionary
    [key: string]: int32;
     ^^^ key
}

interface Mapper {
          ^^^^^^ mapper
    map<Value>(value: Value): Value;
    ^^^ map
        ^^^^^ method_generic
               ^^^^^ parameter
                      ^^^^^ parameter_type
                              ^^^^^ return_type
}

interface Container {
          ^^^^^^^^^ container
    type Item;
         ^^^^ item_declaration
    const Width: usize;
          ^^^^^ width_declaration
}

type Concrete = Container<type Item = string, const Width = 4>;
     ^^^^^^^^ concrete
                ^^^^^^^^^ container_reference
                               ^^^^ item_refinement
                                                    ^^^^^ width_refinement
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#transform type=type modifiers=declaration
@semantic_tokens.token range=main.tspp#source_declaration type=type_parameter modifiers=declaration
@semantic_tokens.token range=main.tspp#mapped_declaration type=type_parameter modifiers=declaration
@semantic_tokens.token range=main.tspp#mapped_source type=type_parameter
@semantic_tokens.token range=main.tspp#mapped_value type=type_parameter
@semantic_tokens.token range=main.tspp#mapped_key type=type_parameter
@semantic_tokens.token range=main.tspp#element type=type modifiers=declaration
@semantic_tokens.token range=main.tspp#value_declaration type=type_parameter modifiers=declaration
@semantic_tokens.token range=main.tspp#value_reference type=type_parameter
@semantic_tokens.token range=main.tspp#infer_declaration type=type_parameter modifiers=declaration
@semantic_tokens.token range=main.tspp#infer_reference type=type_parameter
@semantic_tokens.token range=main.tspp#pair type=type modifiers=declaration
@semantic_tokens.token range=main.tspp#first type=parameter modifiers=declaration
@semantic_tokens.token range=main.tspp#rest type=parameter modifiers=declaration
@semantic_tokens.token range=main.tspp#dictionary type=interface modifiers=declaration
@semantic_tokens.token range=main.tspp#key type=parameter modifiers=declaration
@semantic_tokens.token range=main.tspp#mapper type=interface modifiers=declaration
@semantic_tokens.token range=main.tspp#map type=method modifiers=declaration,abstract
@semantic_tokens.token range=main.tspp#method_generic type=type_parameter modifiers=declaration
@semantic_tokens.token range=main.tspp#parameter type=parameter modifiers=declaration
@semantic_tokens.token range=main.tspp#parameter_type type=type_parameter
@semantic_tokens.token range=main.tspp#return_type type=type_parameter
@semantic_tokens.token range=main.tspp#container type=interface modifiers=declaration
@semantic_tokens.token range=main.tspp#item_declaration type=type modifiers=declaration,abstract
@semantic_tokens.token range=main.tspp#width_declaration type=property modifiers=declaration,readonly,abstract
@semantic_tokens.token range=main.tspp#concrete type=type modifiers=declaration
@semantic_tokens.token range=main.tspp#container_reference type=interface
@semantic_tokens.token range=main.tspp#item_refinement type=type
@semantic_tokens.token range=main.tspp#width_refinement type=property modifiers=readonly
```

### Highlight associated refinement declarations

An associated refinement inherits modifiers from its selected declaration.


```tspp main.tspp
interface Container {
          ^^^^^^^^^ container
    @deprecated("use Element")
     ^^^^^^^^^^ decorator
    type Item;
         ^^^^ declaration
}

type Concrete = Container<type Item = string>;
     ^^^^^^^^ concrete
                ^^^^^^^^^ container_reference
                               ^^^^ refinement
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#container type=interface modifiers=declaration
@semantic_tokens.token range=main.tspp#decorator type=decorator modifiers=default_library
@semantic_tokens.token range=main.tspp#declaration type=type modifiers=declaration,deprecated,abstract
@semantic_tokens.token range=main.tspp#concrete type=type modifiers=declaration
@semantic_tokens.token range=main.tspp#container_reference type=interface
@semantic_tokens.token range=main.tspp#refinement type=type modifiers=deprecated
```

### Highlight object methods and destructuring assignment keys

Object methods declare callable members while assignment keys highlight their selected properties.

```tspp main.tspp
struct Position {
       ^^^^^^^^ position_type
    x: int32;
    ^ x_declaration
    y: int32;
    ^ y_declaration
}

let x = 0;
    ^ x_binding
let y = 0;
    ^ y_binding
const position = Position { x: 1, y: 2 };
      ^^^^^^^^ position_binding
                 ^^^^^^^^ position_reference
                            ^ x_construction
                                  ^ y_construction
({ x: x, y } = position);
   ^ x_key
      ^ x_write
         ^ y_write
               ^^^^^^^^ position_value

const object = {
      ^^^^^^ object
    map<Value>(value: Value): Value {
    ^^^ map
        ^^^^^ generic
               ^^^^^ parameter
                      ^^^^^ parameter_type
                              ^^^^^ return_type
        return value;
               ^^^^^ parameter_reference
    }
};
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#position_type type=struct modifiers=declaration
@semantic_tokens.token range=main.tspp#x_declaration type=property modifiers=declaration
@semantic_tokens.token range=main.tspp#y_declaration type=property modifiers=declaration
@semantic_tokens.token range=main.tspp#x_binding type=variable modifiers=declaration
@semantic_tokens.token range=main.tspp#y_binding type=variable modifiers=declaration
@semantic_tokens.token range=main.tspp#position_binding type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp#position_reference type=struct
@semantic_tokens.token range=main.tspp#x_construction type=property
@semantic_tokens.token range=main.tspp#y_construction type=property
@semantic_tokens.token range=main.tspp#x_key type=property
@semantic_tokens.token range=main.tspp#x_write type=variable modifiers=modification
@semantic_tokens.token range=main.tspp#y_write type=variable modifiers=modification
@semantic_tokens.token range=main.tspp#position_value type=variable modifiers=readonly
@semantic_tokens.token range=main.tspp#object type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp#map type=method modifiers=declaration
@semantic_tokens.token range=main.tspp#generic type=type_parameter modifiers=declaration
@semantic_tokens.token range=main.tspp#parameter type=parameter modifiers=declaration
@semantic_tokens.token range=main.tspp#parameter_type type=type_parameter
@semantic_tokens.token range=main.tspp#return_type type=type_parameter
@semantic_tokens.token range=main.tspp#parameter_reference type=parameter
```

### Highlight tree attributes

Component names and attributes receive function and property highlighting.

```json destack.json
{
  "name": "@test/query",
  "compiler": {
    "tree": "panel.tspp#Panel"
  },
  "targets": {
    "default": {
      "include": ["**/*.tspp"]
    }
  },
  "defaultTarget": "default"
}
```

```tspp panel.tspp
import { TreeBuilder } from "tspp:tree";

export class Panel {}

export extension of Panel implements TreeBuilder {
    type Tags = { span: { title?: string } };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
        tag: Tag,
        attributes: this.Tags[Tag],
        children: Children,
    ): Panel {
        return new Panel();
    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
        return new Panel();
    }
}
```

```tspp main.tspp
import { Panel } from "./panel.tspp";
         ^^^^^ panel_import

function Header(props: { title: string }): Panel {
         ^^^^^^ header
                ^^^^^ parameter
                         ^^^^^ property
                                           ^^^^^ header_result
    return new Panel();
               ^^^^^ panel_constructor
}

const page: Panel = <Header title="hello" />;
      ^^^^ page
            ^^^^^ page_type
                     ^^^^^^ header_reference
                            ^^^^^ attribute
const intrinsic: Panel = <span title="hello" />;
      ^^^^^^^^^ intrinsic
                 ^^^^^ intrinsic_type
                               ^^^^^ intrinsic_attribute
const result = page;
      ^^^^^^ result
               ^^^^ page_reference
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#panel_import type=class modifiers=declaration
@semantic_tokens.token range=main.tspp#header type=function modifiers=declaration
@semantic_tokens.token range=main.tspp#parameter type=parameter modifiers=declaration
@semantic_tokens.token range=main.tspp#property type=property modifiers=declaration
@semantic_tokens.token range=main.tspp#header_result type=class
@semantic_tokens.token range=main.tspp#panel_constructor type=class
@semantic_tokens.token range=main.tspp#page type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp#page_type type=class
@semantic_tokens.token range=main.tspp#header_reference type=function
@semantic_tokens.token range=main.tspp#attribute type=property
@semantic_tokens.token range=main.tspp#intrinsic type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp#intrinsic_type type=class
@semantic_tokens.token range=main.tspp#intrinsic_attribute type=property
@semantic_tokens.token range=main.tspp#result type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp#page_reference type=variable modifiers=readonly
```

### Highlight a declaration while typing

Highlight the document after every inserted character.

```tspp main.tspp
// module
```

```tspp main.tspp type
// module

declare const x: Clone;
              ^ binding
                 ^^^^^ type
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#binding type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp#type type=interface modifiers=default_library
```

### Highlight a selected replacement while typing

Replace the selected type with the first character and insert each remaining character separately.

```tspp main.tspp
declare const value: Wrong;
```

```tspp main.tspp type
declare const value: Clone;
              ^^^^^ binding
                     ^^^^^ type
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#binding type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp#type type=interface modifiers=default_library
```

### Highlight a declaration while backspacing

Highlight the document after every character removed from the end of a type name.

```tspp main.tspp
declare const value: Cloneeeee;
```

```tspp main.tspp backspace
declare const value: Clone;
              ^^^^^ binding
                     ^^^^^ type
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#binding type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp#type type=interface modifiers=default_library
```

### Highlight a declaration while deleting

Highlight the document after every character removed from the start of an identifier.

```tspp main.tspp
declare const temporaryvalue: Clone;
```

```tspp main.tspp delete
declare const value: Clone;
              ^^^^^ binding
                     ^^^^^ type
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#binding type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp#type type=interface modifiers=default_library
```

### Highlight class fields while editing

Class and field declarations remain highlighted through successive edits.

```tspp main.tspp
class Player {}
      ^^^^^^ player
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#player type=class modifiers=declaration
```

```diff main.tspp
@@ -1,2 +1,3 @@
-class Player {}
+class Player {
       ^^^^^^ player
+}
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#player type=class modifiers=declaration
```

```diff main.tspp
@@ -1,3 +1,5 @@
 class Player {
       ^^^^^^ player
+    x:
+    ^ field
 }
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#player type=class modifiers=declaration
@semantic_tokens.token range=main.tspp#field type=property modifiers=declaration
```

```diff main.tspp
@@ -1,5 +1,5 @@
 class Player {
       ^^^^^^ player
-    x:
+    x: number;
     ^ field
 }
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#player type=class modifiers=declaration
@semantic_tokens.token range=main.tspp#field type=property modifiers=declaration
```

### Highlight a struct while typing

Struct declarations and constructions remain highlighted through successive edits.


```tspp main.tspp
struct Position {}
```

```tspp main.tspp type
struct Position {}
       ^^^^^^^^ structure

const position = Position {};
      ^^^^^^^^ binding
                 ^^^^^^^^ construction
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#structure type=struct modifiers=declaration
@semantic_tokens.token range=main.tspp#binding type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp#construction type=struct
```

### Highlight renamed symbols

Renamed declarations and references retain their highlighting.

```tspp main.tspp
function identity(value: int32): int32 {
         ^^^^^^^^ function
                  ^^^^^ parameter
    return value;
           ^^^^^ reference
}
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#function type=function modifiers=declaration
@semantic_tokens.token range=main.tspp#parameter type=parameter modifiers=declaration
@semantic_tokens.token range=main.tspp#reference type=parameter
```

```tspp main.tspp change
function identity(item: int32): int32 {
         ^^^^^^^^ function
                  ^^^^ parameter
    return item;
           ^^^^ reference
}
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#function type=function modifiers=declaration
@semantic_tokens.token range=main.tspp#parameter type=parameter modifiers=declaration
@semantic_tokens.token range=main.tspp#reference type=parameter
```

## Imports

### Highlight imported names and aliases

An imported name and its local alias use the exported declaration kind.

```tspp library.tspp
export function paint(): void {}
```

```tspp main.tspp
import { paint as render } from "./library.tspp";
         ^^^^^ imported
                  ^^^^^^ declaration

render();
^^^^^^ reference
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#imported type=function
@semantic_tokens.token range=main.tspp#declaration type=function modifiers=declaration
@semantic_tokens.token range=main.tspp#reference type=function
```

### Highlight type and namespace imports

Plain and namespace aliases use their target declaration kinds.

```tspp library.tspp
export struct Packet {}
```

```tspp main.tspp
import { Packet as Message } from "./library.tspp";
         ^^^^^^ imported_type
                   ^^^^^^^ type_alias
import * as encoding from "./library.tspp";
            ^^^^^^^^ namespace_alias

declare const message: Message;
              ^^^^^^^ message_declaration
                       ^^^^^^^ type_reference
encoding;
^^^^^^^^ namespace_reference
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#imported_type type=struct
@semantic_tokens.token range=main.tspp#type_alias type=struct modifiers=declaration
@semantic_tokens.token range=main.tspp#namespace_alias type=namespace modifiers=declaration
@semantic_tokens.token range=main.tspp#message_declaration type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp#type_reference type=struct
@semantic_tokens.token range=main.tspp#namespace_reference type=namespace
```

### Highlight nested namespace type paths

Each namespace segment and final type keeps its declaration kind.

```tspp model.tspp
export struct Packet {}
```

```tspp library.tspp
export * as models from "./model";
```

```tspp main.tspp
import * as library from "./library";
            ^^^^^^^ import

declare const packet: library.models.Packet;
              ^^^^^^ binding
                      ^^^^^^^ root
                              ^^^^^^ namespace
                                     ^^^^^^ type
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#import type=namespace modifiers=declaration
@semantic_tokens.token range=main.tspp#binding type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp#root type=namespace
@semantic_tokens.token range=main.tspp#namespace type=namespace
@semantic_tokens.token range=main.tspp#type type=struct
```

### Omit exports without names

Star and default exports without aliases do not carry name tokens.

```tspp library.tspp
export const value = 1;
export default value;
```

```tspp main.tspp
export * from "./library.tspp";
export { default } from "./library.tspp";
export default 1;
```

```query semantic_tokens main.tspp
@semantic_tokens.none
```

## Bindings

### Highlight using bindings

Resource bindings are immutable declarations and references.

```tspp main.tspp
interface Dispose {}
          ^^^^^^^ dispose_interface

declare function open(): Dispose;
                 ^^^^ open_declaration
                         ^^^^^^^ dispose_reference

using resource = open();
      ^^^^^^^^ using_declaration
                 ^^^^ open_reference
resource;
^^^^^^^^ using_reference
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#dispose_interface type=interface modifiers=declaration
@semantic_tokens.token range=main.tspp#open_declaration type=function modifiers=declaration
@semantic_tokens.token range=main.tspp#dispose_reference type=interface
@semantic_tokens.token range=main.tspp#using_declaration type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp#open_reference type=function
@semantic_tokens.token range=main.tspp#using_reference type=variable modifiers=readonly
```

### Highlight tuple match bindings

Tuple patterns bind readonly variables across each arm.

```tspp main.tspp
const pair = (1, 2);
      ^^^^ pair_declaration
const result = match (pair) {
      ^^^^^^ result_declaration
                      ^^^^ pair_reference
    (left, right) if (left > 0) => left + right
     ^^^^ left_declaration
           ^^^^^ right_declaration
                      ^^^^ left_guard_reference
                                   ^^^^ left_reference
                                          ^^^^^ right_reference
};
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#pair_declaration type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp#result_declaration type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp#pair_reference type=variable modifiers=readonly
@semantic_tokens.token range=main.tspp#left_declaration type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp#right_declaration type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp#left_guard_reference type=variable modifiers=readonly
@semantic_tokens.token range=main.tspp#left_reference type=variable modifiers=readonly
@semantic_tokens.token range=main.tspp#right_reference type=variable modifiers=readonly
```

### Highlight nominal object match bindings

A shorthand object pattern binds a readonly variable for its arm.

```tspp main.tspp
struct Box {
       ^^^ box_declaration
    value: int32;
    ^^^^^ property_declaration
}

declare const boxed: Box;
              ^^^^^ boxed_declaration
                     ^^^ box_reference
const result = match (boxed) {
      ^^^^^^ result_declaration
                      ^^^^^ boxed_reference
    Box { value } => value
    ^^^ pattern_type
          ^^^^^ value_declaration
                     ^^^^^ value_reference
};
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#box_declaration type=struct modifiers=declaration
@semantic_tokens.token range=main.tspp#property_declaration type=property modifiers=declaration
@semantic_tokens.token range=main.tspp#boxed_declaration type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp#box_reference type=struct
@semantic_tokens.token range=main.tspp#result_declaration type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp#boxed_reference type=variable modifiers=readonly
@semantic_tokens.token range=main.tspp#pattern_type type=struct
@semantic_tokens.token range=main.tspp#value_declaration type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp#value_reference type=variable modifiers=readonly
```

### Highlight nested object bindings

Object patterns distinguish field names from bindings.

```tspp main.tspp
struct Box {
       ^^^ box_declaration
    value: int32;
    ^^^^^ field_declaration
}

declare const boxed: Box;
              ^^^^^ boxed_declaration
                     ^^^ box_reference
const { value: item, ...rest } = boxed;
        ^^^^^ field_reference
               ^^^^ item_declaration
                        ^^^^ rest_declaration
                                 ^^^^^ boxed_reference
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#box_declaration type=struct modifiers=declaration
@semantic_tokens.token range=main.tspp#field_declaration type=property modifiers=declaration
@semantic_tokens.token range=main.tspp#boxed_declaration type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp#box_reference type=struct
@semantic_tokens.token range=main.tspp#field_reference type=property
@semantic_tokens.token range=main.tspp#item_declaration type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp#rest_declaration type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp#boxed_reference type=variable modifiers=readonly
```

### Highlight member references

Member references use their declaration identities rather than generic property shapes.

```tspp main.tspp
class Buffer {
      ^^^^^^ class_declaration
    readonly length: uint = 0;
             ^^^^^^ field_declaration

    read(offset: uint): uint8 {
    ^^^^ method_declaration
         ^^^^^^ offset_declaration
        return 0;
    }
}

function inspect(buffer: Buffer): uint {
         ^^^^^^^ function_declaration
                 ^^^^^^ buffer_declaration
                         ^^^^^^ type_reference
    const length = buffer.length;
          ^^^^^^ local_declaration
                   ^^^^^^ first_buffer_reference
                          ^^^^^^ field_reference
    buffer.read(0);
    ^^^^^^ second_buffer_reference
           ^^^^ method_reference
    return length;
           ^^^^^^ local_reference
}
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#class_declaration type=class modifiers=declaration
@semantic_tokens.token range=main.tspp#field_declaration type=property modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp#method_declaration type=method modifiers=declaration
@semantic_tokens.token range=main.tspp#offset_declaration type=parameter modifiers=declaration
@semantic_tokens.token range=main.tspp#function_declaration type=function modifiers=declaration
@semantic_tokens.token range=main.tspp#buffer_declaration type=parameter modifiers=declaration
@semantic_tokens.token range=main.tspp#type_reference type=class
@semantic_tokens.token range=main.tspp#local_declaration type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp#first_buffer_reference type=parameter
@semantic_tokens.token range=main.tspp#field_reference type=property modifiers=readonly
@semantic_tokens.token range=main.tspp#second_buffer_reference type=parameter
@semantic_tokens.token range=main.tspp#method_reference type=method
@semantic_tokens.token range=main.tspp#local_reference type=variable modifiers=readonly
```

### Highlight reads and writes

Write occurrences add `modification` without inventing a second mutability modifier.

```tspp main.tspp
struct Counter {
       ^^^^^^^ counter_declaration
    value: int32;
    ^^^^^ field_declaration
}

function increment(counter: Counter): void {
         ^^^^^^^^^ function_declaration
                   ^^^^^^^ counter_parameter
                            ^^^^^^^ counter_type
    let amount = 1;
        ^^^^^^ local_declaration
    amount = amount + 1;
    ^^^^^^ local_write
             ^^^^^^ local_read
    counter.value = amount;
    ^^^^^^^ counter_reference
            ^^^^^ field_write
                    ^^^^^^ amount_read
}
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#counter_declaration type=struct modifiers=declaration
@semantic_tokens.token range=main.tspp#field_declaration type=property modifiers=declaration
@semantic_tokens.token range=main.tspp#function_declaration type=function modifiers=declaration
@semantic_tokens.token range=main.tspp#counter_parameter type=parameter modifiers=declaration
@semantic_tokens.token range=main.tspp#counter_type type=struct
@semantic_tokens.token range=main.tspp#local_declaration type=variable modifiers=declaration
@semantic_tokens.token range=main.tspp#local_write type=variable modifiers=modification
@semantic_tokens.token range=main.tspp#local_read type=variable
@semantic_tokens.token range=main.tspp#counter_reference type=parameter
@semantic_tokens.token range=main.tspp#field_write type=property modifiers=modification
@semantic_tokens.token range=main.tspp#amount_read type=variable
```

## Modifiers

### Highlight symbol attributes

Symbol attributes remain consistent between declarations and references.

```tspp main.tspp
export async function load(): void {}
                      ^^^^ async_declaration

const operation = load;
      ^^^^^^^^^ operation
                  ^^^^ async_reference

abstract class Base {
               ^^^^ abstract_class
    abstract run(): void;
             ^^^ abstract_method
}

class State {
      ^^^^^ state
    static readonly count: int32 = 0;
                    ^^^^^ static_declaration
}

const count = State.count;
      ^^^^^ local_declaration
              ^^^^^ state_reference
                    ^^^^^ static_reference
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#async_declaration type=function modifiers=declaration,async
@semantic_tokens.token range=main.tspp#operation type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp#async_reference type=function modifiers=async
@semantic_tokens.token range=main.tspp#abstract_class type=class modifiers=declaration,abstract
@semantic_tokens.token range=main.tspp#abstract_method type=method modifiers=declaration,abstract
@semantic_tokens.token range=main.tspp#state type=class modifiers=declaration
@semantic_tokens.token range=main.tspp#static_declaration type=property modifiers=declaration,readonly,static
@semantic_tokens.token range=main.tspp#local_declaration type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp#state_reference type=class
@semantic_tokens.token range=main.tspp#static_reference type=property modifiers=readonly,static
```

### Highlight deprecated and default-library symbols

Deprecated state follows the symbol while built-in language items use `default_library`.

```tspp main.tspp
@deprecated("use verify")
 ^^^^^^^^^^ decorator
function validate(): void {}
         ^^^^^^^^ declaration

validate();
^^^^^^^^ reference
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#decorator type=decorator modifiers=default_library
@semantic_tokens.token range=main.tspp#declaration type=function modifiers=declaration,deprecated
@semantic_tokens.token range=main.tspp#reference type=function modifiers=deprecated
```

## Labels and Decorators

### Highlight labels

Label declarations and references receive dedicated highlighting.

```tspp main.tspp
outer: loop {
^^^^^ declaration
    break outer;
          ^^^^^ reference
}
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#declaration type=label modifiers=declaration
@semantic_tokens.token range=main.tspp#reference type=label
```

### Highlight user-defined decorators

Decorator applications remain distinct from the symbols that define them.

```tspp main.tspp
newtype tracked = ();
        ^^^^^^^ annotation_definition

@tracked
 ^^^^^^^ decorator
function start(): void {}
         ^^^^^ function
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#annotation_definition type=type modifiers=declaration
@semantic_tokens.token range=main.tspp#decorator type=decorator
@semantic_tokens.token range=main.tspp#function type=function modifiers=declaration
```

### Highlight interface and parameter decorators

Decorators are highlighted on both an interface method and its parameters.

```tspp main.tspp
newtype tracked = ();
        ^^^^^^^ decorator_type

interface Reader {
          ^^^^^^ interface
    @tracked
     ^^^^^^^ method_decorator
    read(
    ^^^^ method
        @tracked value: string,
         ^^^^^^^ parameter_decorator
                 ^^^^^ parameter
    ): string;
}
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#decorator_type type=type modifiers=declaration
@semantic_tokens.token range=main.tspp#interface type=interface modifiers=declaration
@semantic_tokens.token range=main.tspp#method_decorator type=decorator
@semantic_tokens.token range=main.tspp#method type=method modifiers=declaration,abstract
@semantic_tokens.token range=main.tspp#parameter_decorator type=decorator
@semantic_tokens.token range=main.tspp#parameter type=parameter modifiers=declaration
```

### Highlight struct construction types

Struct expression types identify their nominal constructor.

```tspp main.tspp
struct Point {}
       ^^^^^ point_declaration

const point = Point {};
      ^^^^^ point_binding
              ^^^^^ point_reference
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#point_declaration type=struct modifiers=declaration
@semantic_tokens.token range=main.tspp#point_binding type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp#point_reference type=struct
```

## Literals and Comments

### Preserve literal and comment highlighting

Comments and scalar literals retain their ordinary highlighting.

```tspp main.tspp
// An ordinary comment.
"text";
42;
true;
```

```query semantic_tokens main.tspp
@semantic_tokens.none
```

### Highlight identifier property names

Identifier properties follow their declarations, while quoted names, numeric names, and
`constructor` retain their ordinary highlighting.

```tspp main.tspp
class Container {
      ^^^^^^^^^ container
    constructor() {}
    "quoted"(): void {}
    0: int32 = 0;
    identifier: int32 = 0;
    ^^^^^^^^^^ member
}

type Shape = { "quoted": string; 0: string; identifier: string };
     ^^^^^ shape
                                            ^^^^^^^^^^ type_member

const object = { "quoted": 1, 0: 2, identifier: 3 };
      ^^^^^^ object
                                    ^^^^^^^^^^ property

enum Code {
     ^^^^ code
    Named,
    ^^^^^ named
    "quoted",
    0,
}
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#container type=class modifiers=declaration
@semantic_tokens.token range=main.tspp#member type=property modifiers=declaration
@semantic_tokens.token range=main.tspp#shape type=type modifiers=declaration
@semantic_tokens.token range=main.tspp#type_member type=property modifiers=declaration
@semantic_tokens.token range=main.tspp#object type=variable modifiers=declaration,readonly
@semantic_tokens.token range=main.tspp#property type=property modifiers=declaration
@semantic_tokens.token range=main.tspp#code type=enum modifiers=declaration
@semantic_tokens.token range=main.tspp#named type=enum_member modifiers=declaration,readonly
```

### Highlight documentation while typing

Highlight documentation after every inserted character.

```tspp main.tspp
struct Position {}
```

```tspp main.tspp type
/// A simple position.
^^^^^^^^^^^^^^^^^^^^^^ documentation
struct Position {}
       ^^^^^^^^ position
```

```query semantic_tokens main.tspp
@semantic_tokens.token range=main.tspp#documentation type=comment modifiers=documentation
@semantic_tokens.token range=main.tspp#position type=struct modifiers=declaration
```
