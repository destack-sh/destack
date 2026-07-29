# Search Symbols

## Source changes

### Update search after declarations change

Search follows declarations across revisions.

```ds alpha.ds
export function existingAlpha(): void {}
```

```ds beta.ds
export function existingBeta(): void {}
```

```query search_symbols query=newFeature max_results=10
@search_symbols.none
```

```ds alpha.ds change
export function existingAlpha(): void {}
export function newFeatureAlpha(): void {}
                ^^^^^^^^^^^^^^^ new_feature
```

```ds beta.ds change
export function existingBeta(): void {}
export function newFeatureBeta(): void {}
                ^^^^^^^^^^^^^^ new_feature
```

```query search_symbols query=newFeature max_results=10
@search_symbols.symbol name=newFeatureBeta kind=function location=beta.ds:2:1-2:42 selection=beta.ds#new_feature symbol=beta.ds#newFeatureBeta@2
@search_symbols.symbol name=newFeatureAlpha kind=function location=alpha.ds:2:1-2:43 selection=alpha.ds#new_feature symbol=alpha.ds#newFeatureAlpha@2
```

```diff alpha.ds
@@ -1,3 +1,1 @@
 export function existingAlpha(): void {}
-export function newFeatureAlpha(): void {}
-                ^^^^^^^^^^^^^^^ new_feature
```

```diff beta.ds
@@ -1,3 +1,1 @@
 export function existingBeta(): void {}
-export function newFeatureBeta(): void {}
-                ^^^^^^^^^^^^^^ new_feature
```

```query search_symbols query=newFeature max_results=10
@search_symbols.none
```

### Follow files added, moved, and removed

Search follows the exact workspace file set.

```ds main.ds
export function stableFeature(): void {}
                ^^^^^^^^^^^^^ stable_feature
```

```query search_symbols query=transientFeature max_results=10
@search_symbols.none
```

```ds staged.ds add
export function transientFeature(): void {}
                ^^^^^^^^^^^^^^^^ transient_feature
```

```query search_symbols query=transientFeature max_results=10
@search_symbols.symbol name=transientFeature kind=function location=staged.ds:1:1-1:44 selection=staged.ds#transient_feature symbol=staged.ds#transientFeature@1
```

```query search_symbols query=stableFeature max_results=10
@search_symbols.symbol name=stableFeature kind=function location=main.ds:1:1-1:41 selection=main.ds#stable_feature symbol=main.ds#stableFeature@1
```

```move staged.ds published.ds
```

```query search_symbols query=transientFeature max_results=10
@search_symbols.symbol name=transientFeature kind=function location=published.ds:1:1-1:44 selection=published.ds#transient_feature symbol=published.ds#transientFeature@1
```

```remove published.ds
```

```query search_symbols query=transientFeature max_results=10
@search_symbols.none
```

## Matching

### Search across modules

Symbol search ranks matches across the program.

```ds alpha.ds
export function quartz(): void {}
                ^^^^^^ quartz
export class QuartzClass {}
             ^^^^^^^^^^^ quartz_class
```

```ds beta.ds
export function beta(): void {}
export struct BetaStruct {}
```

```query search_symbols query=quartz max_results=10
@search_symbols.symbol name=quartz kind=function location=alpha.ds:1:1-1:34 selection=alpha.ds#quartz symbol=alpha.ds#quartz@1
@search_symbols.symbol name=QuartzClass kind=class location=alpha.ds:2:1-2:28 selection=alpha.ds#quartz_class symbol=alpha.ds#QuartzClass@2
```

### Match without case sensitivity

Search matching is case-insensitive.

```ds main.ds
export function CamelCaseFeature(): void {}
                ^^^^^^^^^^^^^^^^ name
```

```query search_symbols query=camelcase max_results=10
@search_symbols.symbol name=CamelCaseFeature kind=function location=main.ds:1:1-1:44 selection=main.ds#name symbol=main.ds#CamelCaseFeature@1
```

### Match name boundaries

Initials match camel case and separator boundaries.

```ds main.ds
function getElementsByAttribute(): void {}
         ^^^^^^^^^^^^^^^^^^^^^^ camel_name
function get_element_by_id(): void {}
         ^^^^^^^^^^^^^^^^^ get_name
```

```query search_symbols query=gEA max_results=10
@search_symbols.symbol name=getElementsByAttribute kind=function location=main.ds:1:1-1:43 selection=main.ds#camel_name symbol=main.ds#getElementsByAttribute@1
```

```query search_symbols query=gebi max_results=10
@search_symbols.symbol name=get_element_by_id kind=function location=main.ds:2:1-2:38 selection=main.ds#get_name symbol=main.ds#get_element_by_id@2
@search_symbols.symbol name=getElementsByAttribute kind=function location=main.ds:1:1-1:43 selection=main.ds#camel_name symbol=main.ds#getElementsByAttribute@1
```

### Match ordered subsequences

A compact query may match ordered characters when no stronger lexical match exists.

```ds main.ds
function completionEngine(): void {}
         ^^^^^^^^^^^^^^^^ name
```

```query search_symbols query=cmpl max_results=10
@search_symbols.symbol name=completionEngine kind=function location=main.ds:1:1-1:37 selection=main.ds#name symbol=main.ds#completionEngine@1
```

## Ordering

### Rank exact, prefix, then substring matches

The response follows relevance order.

```ds main.ds
export function orbit(): void {}
                ^^^^^ orbit
export function orbital(): void {}
                ^^^^^^^ orbital
export function megaOrbit(): void {}
                ^^^^^^^^^ mega_orbit
```

```query search_symbols query=orbit max_results=10
@search_symbols.symbol name=orbit kind=function location=main.ds:1:1-1:33 selection=main.ds#orbit symbol=main.ds#orbit@1
@search_symbols.symbol name=orbital kind=function location=main.ds:2:1-2:35 selection=main.ds#orbital symbol=main.ds#orbital@2
@search_symbols.symbol name=megaOrbit kind=function location=main.ds:3:1-3:37 selection=main.ds#mega_orbit symbol=main.ds#megaOrbit@3
```

### Rank duplicate names across modules

Exact names precede prefix matches, and shorter prefixes precede longer prefixes.

```ds alpha.ds
export class Widget {}
             ^^^^^^ widget
export function WidgetFactory(): void {}
                ^^^^^^^^^^^^^ widget_factory
```

```ds beta.ds
export struct Widget {}
              ^^^^^^ widget
export struct WidgetBox {}
              ^^^^^^^^^ widget_box
```

```query search_symbols query=Widget max_results=10
@search_symbols.symbol name=Widget kind=struct location=beta.ds:1:1-1:24 selection=beta.ds#widget symbol=beta.ds#Widget@1
@search_symbols.symbol name=Widget kind=class location=alpha.ds:1:1-1:23 selection=alpha.ds#widget symbol=alpha.ds#Widget@1
@search_symbols.symbol name=WidgetBox kind=struct location=beta.ds:2:1-2:27 selection=beta.ds#widget_box symbol=beta.ds#WidgetBox@2
@search_symbols.symbol name=WidgetFactory kind=function location=alpha.ds:2:1-2:41 selection=alpha.ds#widget_factory symbol=alpha.ds#WidgetFactory@2
```

### Prefer exact case

An exact-case prefix precedes the same prefix after case folding.

```ds main.ds
export function HTTPServer(): void {}
                ^^^^^^^^^^ upper
export function HttpServer(): void {}
                ^^^^^^^^^^ title
```

```query search_symbols query=HT max_results=10
@search_symbols.symbol name=HTTPServer kind=function location=main.ds:1:1-1:38 selection=main.ds#upper symbol=main.ds#HTTPServer@1
@search_symbols.symbol name=HttpServer kind=function location=main.ds:2:1-2:38 selection=main.ds#title symbol=main.ds#HttpServer@2
```

### Limit the ranked results

The result limit truncates the ranked sequence.

```ds main.ds
export function item(): void {}
                ^^^^ item
export function itemize(): void {}
                ^^^^^^^ itemize
export function itemFactory(): void {}
```

```query search_symbols query=item max_results=2
@search_symbols.symbol name=item kind=function location=main.ds:1:1-1:32 selection=main.ds#item symbol=main.ds#item@1
@search_symbols.symbol name=itemize kind=function location=main.ds:2:1-2:35 selection=main.ds#itemize symbol=main.ds#itemize@2
```

## Empty Results

### Return an explicit empty search

An unmatched search returns no symbols.

```ds main.ds
export function alphaOnly(): void {}
```

```query search_symbols query=does_not_exist max_results=10
@search_symbols.none
```

### Respect a zero result limit

A zero result limit returns no symbols without changing matching behavior.

```ds main.ds
function matched(): void {}
```

```query search_symbols query=matched max_results=0
@search_symbols.none
```

## Stable Ordering

### Order identical symbols by their source module

Equal names and kinds follow deterministic module order.

```ds alpha.ds
export function render(): void {}
                ^^^^^^ render
```

```ds beta.ds
export function render(): void {}
                ^^^^^^ render
```

```query search_symbols query=render max_results=10
@search_symbols.symbol name=render kind=function location=beta.ds:1:1-1:34 selection=beta.ds#render symbol=beta.ds#render@1
@search_symbols.symbol name=render kind=function location=alpha.ds:1:1-1:34 selection=alpha.ds#render symbol=alpha.ds#render@1
```

### Order equal members by container

Container names order otherwise equal member matches.

```ds main.ds
class BetaContainer {
    render(): void {}
    ^^^^^^ beta_render
}

class AlphaContainer {
    render(): void {}
    ^^^^^^ alpha_render
}
```

```query search_symbols query=render max_results=10
@search_symbols.symbol name=render kind=method container=AlphaContainer location=main.ds:6:5-6:22 selection=main.ds#alpha_render symbol=main.ds#render@5
@search_symbols.symbol name=render kind=method container=BetaContainer location=main.ds:2:5-2:22 selection=main.ds#beta_render symbol=main.ds#render@2
```

## Members

### Return members with their containers

Member results include the owning nominal type.

```ds main.ds
class Logger {
      ^^^^^^ logger
    log(message: string): void {}
    ^^^ log
}
```

```query search_symbols query=log max_results=10
@search_symbols.symbol name=log kind=method container=Logger location=main.ds:2:5-2:34 selection=main.ds#log symbol=main.ds#log@2
@search_symbols.symbol name=Logger kind=class location=main.ds:1:1-3:2 selection=main.ds#logger symbol=main.ds#Logger@1
```

### Return enum variants with their containers

Enum variants include their owning enum.

```ds main.ds
enum Color {
    Red,
    ^^^ red
}
```

```query search_symbols query=Red max_results=10
@search_symbols.symbol name=Red kind=enum_member container=Color location=main.ds#red symbol=main.ds#Red@2
```

### Distinguish fields and properties

Member results use their editor-facing declaration kinds.

```ds main.ds
class Meter {
    reading: int32;
    ^^^^^^^ reading

    get current(): int32 {
        ^^^^^^^ current
        return this.reading;
    }
}
```

```query search_symbols query=reading max_results=10
@search_symbols.symbol name=reading kind=field container=Meter location=main.ds:2:5-2:19 selection=main.ds#reading symbol=main.ds#reading@2
```

```query search_symbols query=current max_results=10
@search_symbols.symbol name=current kind=property container=Meter location=main.ds:4:5-6:6 selection=main.ds#current symbol=main.ds#current@4
```

## Scope

### Return non-exported module declarations

Program search includes named module declarations regardless of export visibility.

```ds main.ds
function internalSearch(): void {}
         ^^^^^^^^^^^^^^ name
```

```query search_symbols query=internalSearch max_results=10
@search_symbols.symbol name=internalSearch kind=function location=main.ds:1:1-1:35 selection=main.ds#name symbol=main.ds#internalSearch@1
```

### Omit local bindings and parameters

Function parameters and body-local bindings do not participate in program search.

```ds main.ds
function calculate(searchInput: int32): int32 {
    const searchLocal = searchInput;
    return searchLocal;
}
```

```query search_symbols query=searchInput max_results=10
@search_symbols.none
```

```query search_symbols query=searchLocal max_results=10
@search_symbols.none
```

### Omit import bindings

Search returns the declaring symbol without duplicating an importing binding.

```ds library.ds
export function externalSearch(): void {}
                ^^^^^^^^^^^^^^ name
```

```ds main.ds
import { externalSearch } from "./library.ds";
```

```query search_symbols query=externalSearch max_results=10
@search_symbols.symbol name=externalSearch kind=function location=library.ds:1:1-1:42 selection=library.ds#name symbol=library.ds#externalSearch@1
```

## Declaration Kinds

### Return a type alias

Type aliases participate in symbol search.

```ds main.ds
export type UserId = string;
            ^^^^^^ name
```

```query search_symbols query=UserId max_results=10
@search_symbols.symbol name=UserId kind=type_alias location=main.ds:1:1-1:28 selection=main.ds#name symbol=main.ds#UserId@1
```

### Return an interface

Interfaces participate in symbol search.

```ds main.ds
export interface SearchInterface {}
                 ^^^^^^^^^^^^^^^ name
```

```query search_symbols query=SearchInterface max_results=10
@search_symbols.symbol name=SearchInterface kind=interface location=main.ds:1:1-1:36 selection=main.ds#name symbol=main.ds#SearchInterface@1
```

### Return an enum

Enums participate in symbol search.

```ds main.ds
export enum SearchState { Ready }
            ^^^^^^^^^^^ name
```

```query search_symbols query=SearchState max_results=10
@search_symbols.symbol name=SearchState kind=enum location=main.ds:1:1-1:34 selection=main.ds#name symbol=main.ds#SearchState@1
```

### Return a newtype

Newtypes participate in symbol search.

```ds main.ds
export newtype SearchId = uint64;
               ^^^^^^^^ name
```

```query search_symbols query=SearchId max_results=10
@search_symbols.symbol name=SearchId kind=newtype location=main.ds:1:1-1:33 selection=main.ds#name symbol=main.ds#SearchId@1
```

### Return a nominal interface

Nominal interfaces participate in symbol search.

```ds main.ds
export newtype interface SearchCapability {}
                         ^^^^^^^^^^^^^^^^ name
```

```query search_symbols query=SearchCapability max_results=10
@search_symbols.symbol name=SearchCapability kind=newtype_interface location=main.ds:1:1-1:45 selection=main.ds#name symbol=main.ds#SearchCapability@1
```

### Return a named extension

Named extensions participate in symbol search.

```ds main.ds
export struct SearchSubject {}
export extension SearchExtension of SearchSubject {}
                 ^^^^^^^^^^^^^^^ name
```

```query search_symbols query=SearchExtension max_results=10
@search_symbols.symbol name=SearchExtension kind=extension location=main.ds:2:1-2:53 selection=main.ds#name symbol=main.ds#SearchExtension@2
```

### Distinguish constants and variables

Top-level value declarations use mutability-derived editor kinds.

```ds main.ds
export const searchValue = 1;
             ^^^^^^^^^^^ name

export let mutableSearchValue = 2;
           ^^^^^^^^^^^^^^^^^^ mutable_name
```

```query search_symbols query=searchValue max_results=10
@search_symbols.symbol name=searchValue kind=constant location=main.ds#name symbol=main.ds#searchValue@1
@search_symbols.symbol name=mutableSearchValue kind=variable location=main.ds#mutable_name symbol=main.ds#mutableSearchValue@2
```
