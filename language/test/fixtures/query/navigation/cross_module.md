# Cross Module Navigation

## Imported Symbols

### Goto definition across modules

Goto definition should resolve imported symbols across files.

The library module provides the exported symbol.

```ds:lib.ds
export function greet(name: string): string {
//                ^^^^^ def:greet
    return "Hello, " + name;
}
```

The main module imports and uses the symbol.

```ds:main.ds
import { greet } from "./lib.ds";

const message = greet("Destack");
//      ^^^^^^^ def:message
//                ^^^^^ use:greet

const local = message;
//              ^^^^^^^ use:message
```

Goto definition from the imported call should land on the exported definition.

```query goto_definition use:greet
def:greet
```

Find references should include the definition, import specifier, and call site.

```query find_references use:greet
lib.ds:1:17-1:22
main.ds:1:10-1:15
main.ds:3:17-3:22
```

Goto definition should still handle local symbols in the same file.

```query goto_definition use:message
def:message
```

## Import Shapes

### Goto definition through aliased imports

Goto definition should still reach the exported definition through a local import alias.

```ds:alias_lib.ds
export function greet(name: string): string {
//                ^^^^^ def:greet
    return "Hello, " + name;
}
```

```ds:alias_main.ds
import { greet as importedGreet } from "./alias_lib.ds";

const greet = 1;
const message = importedGreet("Destack");
//                ^^^^^^^^^^^^^ use:importedGreet
```

```query goto_definition use:importedGreet
def:greet
```

### Goto definition through namespace imports

Goto definition should resolve through namespace member access across modules.

```ds:namespace_lib.ds
export function greet(name: string): string {
//                ^^^^^ def:namespace_greet
    return "Hello, " + name;
}
```

```ds:namespace_main.ds
import * as api from "./namespace_lib.ds";

const message = api.greet("Destack");
//                    ^^^^^ use:namespace_greet
```

```query goto_definition use:namespace_greet
def:namespace_greet
```

### Resolve definitions through default imports

Goto definition should resolve default imports across modules.

```ds:default_lib.ds
export default function greetDefault(name: string): string {
//                        ^^^^^^^^^^^^ def:default_greet
    return "Hello, " + name;
}
```

```ds:default_main.ds
import greetDefault from "./default_lib.ds";
//       ^^^^^^^^^^^^ decl:default_greet_import

const message = greetDefault("Destack");
//                ^^^^^^^^^^^^ use:default_greet
```

```query goto_declaration use:default_greet
decl:default_greet_import
```

```query goto_definition use:default_greet
def:default_greet
```

### Resolve declarations and definitions through mixed type imports

Cross-module navigation should still resolve local declarations and source definitions for aliased type-only imports.

```ds:type_lib.ds
export type Settings = {
//            ^^^^^^^^ def:aliased_settings
    mode: string,
};
```

```ds:type_main.ds
import type { Settings as AppSettings } from "./type_lib.ds";
//                          ^^^^^^^^^^^ decl:AppSettings_import

function configure(settings: AppSettings): void {
//                             ^^^^^^^^^^^ use:AppSettings
    console.log(settings.mode);
}
```

```query goto_declaration use:AppSettings
decl:AppSettings_import
```

```query goto_definition use:AppSettings
def:aliased_settings
```

```query goto_type_definition use:AppSettings
def:aliased_settings
```

## Mixed Query Surfaces

### Resolve definitions, declarations, and references across mixed import shapes

Cross-module navigation should stay correct when named, aliased, namespace, and type-only imports all coexist.

```ds:types.ds
export type Settings = {
//            ^^^^^^^^ def:Settings
    mode: string,
};
```

```ds:values.ds
export function greet(name: string): string {
//                ^^^^^ def:mixed_greet
    return "Hello, " + name;
}
```

```ds:barrel.ds
export { greet as sayHello } from "./values.ds";
export * as api from "./values.ds";
export type { Settings } from "./types.ds";
```

```ds:main.ds
import { sayHello } from "./barrel.ds";
//         ^^^^^^^^ decl:sayHello_import
import { api } from "./barrel.ds";
import type { Settings } from "./barrel.ds";
//              ^^^^^^^^ decl:Settings_import

const config: Settings = { mode: "friendly" };
//              ^^^^^^^^ use:Settings

const one = sayHello("Destack");
//            ^^^^^^^^ use:sayHello

const two = api.greet("Destack");
//                ^^^^^ use:api_greet
```

```query goto_declaration use:sayHello
decl:sayHello_import
```

```query goto_definition use:sayHello
def:mixed_greet
```

```query goto_definition use:Settings
def:Settings
```

```query goto_type_definition use:Settings
def:Settings
```

```query goto_definition use:api_greet
def:mixed_greet
```

```query goto_declaration use:Settings
decl:Settings_import
```

```query find_references def:mixed_greet
values.ds:1:17-1:22
barrel.ds:1:10-1:15
main.ds:1:10-1:18
main.ds:7:13-7:21
main.ds:9:17-9:22
```

### Resolve mixed value and type imports through default and namespace barrels

Cross-module navigation should still target the right declarations when default re-export aliases, namespace barrels, and local shadows coexist.

```ds:widget.ds
export default class WidgetModel {}
//                     ^^^^^^^^^^^ def:WidgetModel
```

```ds:helpers.ds
export function buildWidget(): WidgetModel {
//                ^^^^^^^^^^^ def:buildWidget_barrel
    return new WidgetModel();
}
```

```ds:barrel.ds
export { default as Widget } from "./widget.ds";
export * as helpers from "./helpers.ds";
```

```ds:main.ds
import { Widget as WidgetType } from "./barrel.ds";
//                   ^^^^^^^^^^ decl:Widget_import
import { helpers } from "./barrel.ds";

const Widget = 1;
const model: WidgetType = helpers.buildWidget();
//             ^^^^^^^^^^ use:Widget_type
//                                  ^^^^^^^^^^^ use:buildWidget_barrel
```

```query goto_type_definition use:Widget_type
def:WidgetModel
```

```query goto_definition use:Widget_type
def:WidgetModel
```

```query goto_declaration use:Widget_type
decl:Widget_import
```

```query goto_definition use:buildWidget_barrel
def:buildWidget_barrel
```
