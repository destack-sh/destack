
## Matching

### Search across modules

Symbol search ranks matches across the program.

```tspp alpha.tspp
export function quartz(): void {}
                ^^^^^^ quartz
export class QuartzClass {}
             ^^^^^^^^^^^ quartz_class
```

```tspp beta.tspp
export function beta(): void {}
export struct BetaStruct {}
```

```query search_symbols query=quartz max_results=10
@search_symbols.symbol name=quartz kind=function location=alpha.tspp:1:1-1:34 selection=alpha.tspp#quartz symbol=alpha.tspp#quartz@1
@search_symbols.symbol name=QuartzClass kind=class location=alpha.tspp:2:1-2:28 selection=alpha.tspp#quartz_class symbol=alpha.tspp#QuartzClass@2
```

### Match without case sensitivity

Search matching is case-insensitive.

```tspp main.tspp
export function CamelCaseFeature(): void {}
                ^^^^^^^^^^^^^^^^ name
```

```query search_symbols query=camelcase max_results=10
@search_symbols.symbol name=CamelCaseFeature kind=function location=main.tspp:1:1-1:44 selection=main.tspp#name symbol=main.tspp#CamelCaseFeature@1
```

### Match name boundaries

Initials match camel case and separator boundaries.

```tspp main.tspp
function getElementsByAttribute(): void {}
         ^^^^^^^^^^^^^^^^^^^^^^ camel_name
function get_element_by_id(): void {}
         ^^^^^^^^^^^^^^^^^ get_name
```

```query search_symbols query=gEA max_results=10
@search_symbols.symbol name=getElementsByAttribute kind=function location=main.tspp:1:1-1:43 selection=main.tspp#camel_name symbol=main.tspp#getElementsByAttribute@1
```

```query search_symbols query=gebi max_results=10
@search_symbols.symbol name=get_element_by_id kind=function location=main.tspp:2:1-2:38 selection=main.tspp#get_name symbol=main.tspp#get_element_by_id@2
@search_symbols.symbol name=getElementsByAttribute kind=function location=main.tspp:1:1-1:43 selection=main.tspp#camel_name symbol=main.tspp#getElementsByAttribute@1
```

### Match ordered subsequences

A compact search term may match ordered characters when no stronger lexical match exists.

```tspp main.tspp
function completionEngine(): void {}
         ^^^^^^^^^^^^^^^^ name
```

```query search_symbols query=cmpl max_results=10
@search_symbols.symbol name=completionEngine kind=function location=main.tspp:1:1-1:37 selection=main.tspp#name symbol=main.tspp#completionEngine@1
```

## Ordering

### Rank exact, prefix, then substring matches

Matching symbols follow relevance order.

```tspp main.tspp
export function orbit(): void {}
                ^^^^^ orbit
export function orbital(): void {}
                ^^^^^^^ orbital
export function megaOrbit(): void {}
                ^^^^^^^^^ mega_orbit
```

```query search_symbols query=orbit max_results=10
@search_symbols.symbol name=orbit kind=function location=main.tspp:1:1-1:33 selection=main.tspp#orbit symbol=main.tspp#orbit@1
@search_symbols.symbol name=orbital kind=function location=main.tspp:2:1-2:35 selection=main.tspp#orbital symbol=main.tspp#orbital@2
@search_symbols.symbol name=megaOrbit kind=function location=main.tspp:3:1-3:37 selection=main.tspp#mega_orbit symbol=main.tspp#megaOrbit@3
```

### Rank duplicate names across modules

Exact names precede prefix matches, and shorter prefixes precede longer prefixes.

```tspp alpha.tspp
export class Widget {}
             ^^^^^^ widget
export function WidgetFactory(): void {}
                ^^^^^^^^^^^^^ widget_factory
```

```tspp beta.tspp
export struct Widget {}
              ^^^^^^ widget
export struct WidgetBox {}
              ^^^^^^^^^ widget_box
```

```query search_symbols query=Widget max_results=10
@search_symbols.symbol name=Widget kind=struct location=beta.tspp:1:1-1:24 selection=beta.tspp#widget symbol=beta.tspp#Widget@1
@search_symbols.symbol name=Widget kind=class location=alpha.tspp:1:1-1:23 selection=alpha.tspp#widget symbol=alpha.tspp#Widget@1
@search_symbols.symbol name=WidgetBox kind=struct location=beta.tspp:2:1-2:27 selection=beta.tspp#widget_box symbol=beta.tspp#WidgetBox@2
@search_symbols.symbol name=WidgetFactory kind=function location=alpha.tspp:2:1-2:41 selection=alpha.tspp#widget_factory symbol=alpha.tspp#WidgetFactory@2
```

### Prefer exact case

An exact-case prefix precedes the same prefix after case folding.

```tspp main.tspp
export function HTTPServer(): void {}
                ^^^^^^^^^^ upper
export function HttpServer(): void {}
                ^^^^^^^^^^ title
```

```query search_symbols query=HT max_results=10
@search_symbols.symbol name=HTTPServer kind=function location=main.tspp:1:1-1:38 selection=main.tspp#upper symbol=main.tspp#HTTPServer@1
@search_symbols.symbol name=HttpServer kind=function location=main.tspp:2:1-2:38 selection=main.tspp#title symbol=main.tspp#HttpServer@2
```

### Limit the ranked results

The result limit truncates the ranked sequence.

```tspp main.tspp
export function item(): void {}
                ^^^^ item
export function itemize(): void {}
                ^^^^^^^ itemize
export function itemFactory(): void {}
```

```query search_symbols query=item max_results=2
@search_symbols.symbol name=item kind=function location=main.tspp:1:1-1:32 selection=main.tspp#item symbol=main.tspp#item@1
@search_symbols.symbol name=itemize kind=function location=main.tspp:2:1-2:35 selection=main.tspp#itemize symbol=main.tspp#itemize@2
```

## Empty Results

### Return an explicit empty search

An unmatched search returns no symbols.

```tspp main.tspp
export function alphaOnly(): void {}
```

```query search_symbols query=does_not_exist max_results=10
@search_symbols.none
```

### Respect a zero result limit

A zero result limit returns no symbols without changing matching behavior.

```tspp main.tspp
function matched(): void {}
```

```query search_symbols query=matched max_results=0
@search_symbols.none
```

## Stable Ordering

### Order identical symbols by their source module

Equal names and kinds follow deterministic module order.

```tspp alpha.tspp
export function render(): void {}
                ^^^^^^ render
```

```tspp beta.tspp
export function render(): void {}
                ^^^^^^ render
```

```query search_symbols query=render max_results=10
@search_symbols.symbol name=render kind=function location=beta.tspp:1:1-1:34 selection=beta.tspp#render symbol=beta.tspp#render@1
@search_symbols.symbol name=render kind=function location=alpha.tspp:1:1-1:34 selection=alpha.tspp#render symbol=alpha.tspp#render@1
```

### Order equal members by container

Container names order otherwise equal member matches.

```tspp main.tspp
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
@search_symbols.symbol name=render kind=method container=AlphaContainer location=main.tspp:6:5-6:22 selection=main.tspp#alpha_render symbol=main.tspp#render@5
@search_symbols.symbol name=render kind=method container=BetaContainer location=main.tspp:2:5-2:22 selection=main.tspp#beta_render symbol=main.tspp#render@2
```

## Members

### Return members with their containers

Member results include the owning nominal type.

```tspp main.tspp
class Logger {
      ^^^^^^ logger
    log(message: string): void {}
    ^^^ log
}
```

```query search_symbols query=log max_results=10
@search_symbols.symbol name=log kind=method container=Logger location=main.tspp:2:5-2:34 selection=main.tspp#log symbol=main.tspp#log@2
@search_symbols.symbol name=Logger kind=class location=main.tspp:1:1-3:2 selection=main.tspp#logger symbol=main.tspp#Logger@1
```

### Return enum variants with their containers

Enum variants include their owning enum.

```tspp main.tspp
enum Color {
    Red,
    ^^^ red
}
```

```query search_symbols query=Red max_results=10
@search_symbols.symbol name=Red kind=enum_member container=Color location=main.tspp#red symbol=main.tspp#Red@2
```

### Distinguish fields and properties

Member results use their editor-facing declaration kinds.

```tspp main.tspp
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
@search_symbols.symbol name=reading kind=field container=Meter location=main.tspp:2:5-2:19 selection=main.tspp#reading symbol=main.tspp#reading@2
```

```query search_symbols query=current max_results=10
@search_symbols.symbol name=current kind=property container=Meter location=main.tspp:4:5-6:6 selection=main.tspp#current symbol=main.tspp#current@4
```

## Scope

### Return non-exported module declarations

Program search includes named module declarations regardless of export visibility.

```tspp main.tspp
function internalSearch(): void {}
         ^^^^^^^^^^^^^^ name
```

```query search_symbols query=internalSearch max_results=10
@search_symbols.symbol name=internalSearch kind=function location=main.tspp:1:1-1:35 selection=main.tspp#name symbol=main.tspp#internalSearch@1
```

### Omit local bindings and parameters

Function parameters and body-local bindings do not participate in program search.

```tspp main.tspp
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

```tspp library.tspp
export function externalSearch(): void {}
                ^^^^^^^^^^^^^^ name
```

```tspp main.tspp
import { externalSearch } from "./library.tspp";
```

```query search_symbols query=externalSearch max_results=10
@search_symbols.symbol name=externalSearch kind=function location=library.tspp:1:1-1:42 selection=library.tspp#name symbol=library.tspp#externalSearch@1
```

### Return named re-export aliases

Named re-exports use the target declaration kind at the alias declaration.

```tspp library.tspp
export function internalRender(): void {}
export const internalLimit = 10;
```

```tspp barrel.tspp
export { internalRender as publicRender } from "./library.tspp";
                           ^^^^^^^^^^^^ name
export { internalLimit as publicLimit } from "./library.tspp";
                          ^^^^^^^^^^^ public_limit
```

```query search_symbols query=publicRender max_results=10
@search_symbols.symbol name=publicRender kind=function location=barrel.tspp:1:10-1:40 selection=barrel.tspp#name symbol=barrel.tspp#publicRender@1
```

```query search_symbols query=publicLimit max_results=10
@search_symbols.symbol name=publicLimit kind=constant location=barrel.tspp:2:10-2:38 selection=barrel.tspp#public_limit symbol=barrel.tspp#publicLimit@2
```

### Resolve re-export alias chains

Re-export chains use the final declaration kind at the outer alias.

```tspp library.tspp
export function internalRender(): void {}
```

```tspp intermediate.tspp
export { internalRender as sharedRender } from "./library.tspp";
```

```tspp barrel.tspp
export { sharedRender as publicRender } from "./intermediate.tspp";
                         ^^^^^^^^^^^^ name
```

```query search_symbols query=publicRender max_results=10
@search_symbols.symbol name=publicRender kind=function location=barrel.tspp:1:10-1:38 selection=barrel.tspp#name symbol=barrel.tspp#publicRender@1
```

### Return namespace re-export aliases

Namespace re-exports retain their authored alias and module kind.

```tspp library.tspp
export function execute(): void {}
```

```tspp barrel.tspp
export * as publicApi from "./library.tspp";
            ^^^^^^^^^ name
```

```query search_symbols query=publicApi max_results=10
@search_symbols.symbol name=publicApi kind=namespace location=barrel.tspp:1:8-1:42 selection=barrel.tspp#name symbol=barrel.tspp#publicApi@1
```

### Return no unresolved re-export alias

An unresolved alias has no editor symbol kind.

```tspp library.tspp
export function available(): void {}
```

```tspp barrel.tspp
export { missing as publicMissing } from "./library.tspp";
```

```query search_symbols query=publicMissing max_results=10
@search_symbols.none
```

### Search current workspace files

Search follows the exact workspace file set.

```tspp main.tspp
export function stableFeature(): void {}
                ^^^^^^^^^^^^^ stable_feature
```

```query search_symbols query=transientFeature max_results=10
@search_symbols.none
```

```tspp staged.tspp add
export function transientFeature(): void {}
                ^^^^^^^^^^^^^^^^ transient_feature
```

```query search_symbols query=transientFeature max_results=10
@search_symbols.symbol name=transientFeature kind=function location=staged.tspp:1:1-1:44 selection=staged.tspp#transient_feature symbol=staged.tspp#transientFeature@1
```

```query search_symbols query=stableFeature max_results=10
@search_symbols.symbol name=stableFeature kind=function location=main.tspp:1:1-1:41 selection=main.tspp#stable_feature symbol=main.tspp#stableFeature@1
```

```move staged.tspp published.tspp
```

```query search_symbols query=transientFeature max_results=10
@search_symbols.symbol name=transientFeature kind=function location=published.tspp:1:1-1:44 selection=published.tspp#transient_feature symbol=published.tspp#transientFeature@1
```

```remove published.tspp
```

```query search_symbols query=transientFeature max_results=10
@search_symbols.none
```

## Declaration Kinds

### Return a type alias

Type aliases participate in symbol search.

```tspp main.tspp
export type UserId = string;
            ^^^^^^ name
```

```query search_symbols query=UserId max_results=10
@search_symbols.symbol name=UserId kind=type_alias location=main.tspp:1:1-1:28 selection=main.tspp#name symbol=main.tspp#UserId@1
```

### Return an interface

Interfaces participate in symbol search.

```tspp main.tspp
export interface SearchInterface {}
                 ^^^^^^^^^^^^^^^ name
```

```query search_symbols query=SearchInterface max_results=10
@search_symbols.symbol name=SearchInterface kind=interface location=main.tspp:1:1-1:36 selection=main.tspp#name symbol=main.tspp#SearchInterface@1
```

### Return an enum

Enums participate in symbol search.

```tspp main.tspp
export enum SearchState { Ready }
            ^^^^^^^^^^^ name
```

```query search_symbols query=SearchState max_results=10
@search_symbols.symbol name=SearchState kind=enum location=main.tspp:1:1-1:34 selection=main.tspp#name symbol=main.tspp#SearchState@1
```

### Return a newtype

Newtypes participate in symbol search.

```tspp main.tspp
export newtype SearchId = uint64;
               ^^^^^^^^ name
```

```query search_symbols query=SearchId max_results=10
@search_symbols.symbol name=SearchId kind=newtype location=main.tspp:1:1-1:33 selection=main.tspp#name symbol=main.tspp#SearchId@1
```

### Return a nominal interface

Nominal interfaces participate in symbol search.

```tspp main.tspp
export newtype interface SearchCapability {}
                         ^^^^^^^^^^^^^^^^ name
```

```query search_symbols query=SearchCapability max_results=10
@search_symbols.symbol name=SearchCapability kind=newtype_interface location=main.tspp:1:1-1:45 selection=main.tspp#name symbol=main.tspp#SearchCapability@1
```

### Return a named extension

Named extensions participate in symbol search.

```tspp main.tspp
export struct SearchSubject {}
export extension SearchExtension of SearchSubject {}
                 ^^^^^^^^^^^^^^^ name
```

```query search_symbols query=SearchExtension max_results=10
@search_symbols.symbol name=SearchExtension kind=extension location=main.tspp:2:1-2:53 selection=main.tspp#name symbol=main.tspp#SearchExtension@2
```

### Distinguish constants and variables

Top-level value declarations use mutability-derived editor kinds.

```tspp main.tspp
export const searchValue = 1;
             ^^^^^^^^^^^ name

export let mutableSearchValue = 2;
           ^^^^^^^^^^^^^^^^^^ mutable_name
```

```query search_symbols query=searchValue max_results=10
@search_symbols.symbol name=searchValue kind=constant location=main.tspp#name symbol=main.tspp#searchValue@1
@search_symbols.symbol name=mutableSearchValue kind=variable location=main.tspp#mutable_name symbol=main.tspp#mutableSearchValue@2
```

### Search current declarations

Search follows declarations across edits.

```tspp alpha.tspp
export function existingAlpha(): void {}
```

```tspp beta.tspp
export function existingBeta(): void {}
```

```query search_symbols query=newFeature max_results=10
@search_symbols.none
```

```tspp alpha.tspp change
export function existingAlpha(): void {}
export function newFeatureAlpha(): void {}
                ^^^^^^^^^^^^^^^ new_feature
```

```tspp beta.tspp change
export function existingBeta(): void {}
export function newFeatureBeta(): void {}
                ^^^^^^^^^^^^^^ new_feature
```

```query search_symbols query=newFeature max_results=10
@search_symbols.symbol name=newFeatureBeta kind=function location=beta.tspp:2:1-2:42 selection=beta.tspp#new_feature symbol=beta.tspp#newFeatureBeta@2
@search_symbols.symbol name=newFeatureAlpha kind=function location=alpha.tspp:2:1-2:43 selection=alpha.tspp#new_feature symbol=alpha.tspp#newFeatureAlpha@2
```

```diff alpha.tspp
@@ -1,3 +1,1 @@
 export function existingAlpha(): void {}
-export function newFeatureAlpha(): void {}
-                ^^^^^^^^^^^^^^^ new_feature
```

```diff beta.tspp
@@ -1,3 +1,1 @@
 export function existingBeta(): void {}
-export function newFeatureBeta(): void {}
-                ^^^^^^^^^^^^^^ new_feature
```

```query search_symbols query=newFeature max_results=10
@search_symbols.none
```
