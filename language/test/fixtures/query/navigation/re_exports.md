# Re-Export Navigation

## Re-Exported Symbols

### Goto definition through re-exports

Goto definition should resolve symbols through re-export chains.

```ds:base.ds
export function greet(name: string): string {
//                ^^^^^ def:greet
    return "Hello, " + name;
}
```

```ds:reexport.ds
export { greet } from "./base.ds";
```

```ds:main.ds
import { greet } from "./reexport.ds";

const message = greet("Destack");
//                ^^^^^ use:greet
```

```query goto_definition use:greet
def:greet
```

### Goto definition through re-exported aliases

Re-exported aliases should resolve to the original definition.

```ds:alias_base.ds
export function build(name: string): string {
//                ^^^^^ def:build
    return "Hello, " + name;
}
```

```ds:alias_reexport.ds
export { build as buildAlias } from "./alias_base.ds";
```

```ds:alias_main.ds
import { buildAlias } from "./alias_reexport.ds";

const message = buildAlias("Destack");
//                ^^^^^^^^^^ use:build_alias
```

```query goto_definition use:build_alias
def:build
```

### Goto definition through export star chains

Export star chains should resolve to the original definition.

```ds:base_star.ds
export function wave(name: string): string {
//                ^^^^ def:wave
    return "Hi, " + name;
}
```

```ds:barrel_one.ds
export * from "./base_star.ds";
```

```ds:barrel_two.ds
export * from "./barrel_one.ds";
```

```ds:main_star.ds
import { wave } from "./barrel_two.ds";

const message = wave("Destack");
//                ^^^^ use:wave
```

```query goto_definition use:wave
def:wave
```

### Goto definition for default exports

Default imports should resolve to the exported definition.

```ds:base_default.ds
export default function greetDefault(name: string): string {
//                        ^^^^^^^^^^^^ def:greet_default
    return "Hello, " + name;
}
```

```ds:main_default.ds
import greetDefault from "./base_default.ds";

const message = greetDefault("Destack");
//                ^^^^^^^^^^^^ use:greet_default
```

```query goto_definition use:greet_default
def:greet_default
```

### Goto definition through type-only re-export chains

Type-only re-export chains should still resolve to the original type declaration.

```ds:types.ds
export type Config = string;
//            ^^^^^^ def:Config
```

```ds:barrel_a.ds
export type { Config as AppConfig } from "./types.ds";
```

```ds:barrel_b.ds
export type { AppConfig } from "./barrel_a.ds";
```

```ds:main_type.ds
import type { AppConfig } from "./barrel_b.ds";

const config: AppConfig = "ok";
//              ^^^^^^^^^ use:AppConfig
```

```query goto_definition use:AppConfig
def:Config
```

### Goto definition through export namespace chains

Exported namespaces should preserve member definitions through re-export chains.

```ds:ns_base.ds
export function paint(color: string): string {
//                ^^^^^ def:ns_paint
    return color;
}
```

```ds:ns_barrel_a.ds
export * as palette from "./ns_base.ds";
```

```ds:ns_barrel_b.ds
export { palette } from "./ns_barrel_a.ds";
```

```ds:ns_main.ds
import { palette } from "./ns_barrel_b.ds";

const value = palette.paint("blue");
//                      ^^^^^ use:ns_paint
```

```query goto_definition use:ns_paint
def:ns_paint
```

## Mixed Query Surfaces

### Re-export chains should preserve declarations and references

Re-export chains should support declaration and reference queries in addition to definition lookup.

```ds:base.ds
export function buildWidget(): string {
//                ^^^^^^^^^^^ def:buildWidget
    return "ok";
}
```

```ds:barrel_a.ds
export { buildWidget as makeWidget } from "./base.ds";
```

```ds:barrel_b.ds
export { makeWidget } from "./barrel_a.ds";
```

```ds:main.ds
import { makeWidget } from "./barrel_b.ds";
//         ^^^^^^^^^^ decl:makeWidget_import

const value = makeWidget();
//              ^^^^^^^^^^ use:makeWidget
```

```query goto_declaration use:makeWidget
decl:makeWidget_import
```

```query goto_definition use:makeWidget
def:buildWidget
```

```query find_references def:buildWidget
base.ds:1:17-1:28
barrel_a.ds:1:10-1:21
barrel_b.ds:1:10-1:20
main.ds:1:10-1:20
main.ds:3:15-3:25
```

### Goto type definition through default class re-export aliases

Default-exported class aliases should still resolve to the original type declaration.

```ds:model.ds
export default class WidgetModel {}
//                     ^^^^^^^^^^^ def:WidgetModel
```

```ds:barrel.ds
export { default as Widget } from "./model.ds";
```

```ds:main.ds
import { Widget } from "./barrel.ds";
//         ^^^^^^ decl:Widget_import

const value: Widget = new Widget();
//             ^^^^^^ use:Widget
```

```query goto_declaration use:Widget
decl:Widget_import
```

```query goto_definition use:Widget
def:WidgetModel
```

```query goto_type_definition use:Widget
def:WidgetModel
```

### Re-export chains should preserve type-space declarations

Type-only re-export aliases should still stop at the local import declaration while resolving type definitions to the source declaration.

```ds:types.ds
export type Settings = {
//            ^^^^^^^^ def:reexport_settings
    enabled: boolean,
};
```

```ds:barrel_a.ds
export type { Settings as AppSettings } from "./types.ds";
```

```ds:barrel_b.ds
export type { AppSettings } from "./barrel_a.ds";
```

```ds:main.ds
import type { AppSettings } from "./barrel_b.ds";
//              ^^^^^^^^^^^ decl:AppSettings_import

const config: AppSettings = { enabled: true };
//              ^^^^^^^^^^^ use:AppSettings
```

```query goto_declaration use:AppSettings
decl:AppSettings_import
```

```query goto_definition use:AppSettings
def:reexport_settings
```

```query goto_type_definition use:AppSettings
def:reexport_settings
```
