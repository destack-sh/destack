
## Imports

### Link resolved import specifiers

Each resolved import specifier becomes a link in source order.

```tspp main.tspp
import { foo } from "./foo.tspp";
                    ^^^^^^^^^^ foo_specifier
import { bar } from "./bar.tspp";
                    ^^^^^^^^^^ bar_specifier
```

```tspp foo.tspp
export const foo = 1;
```

```tspp bar.tspp
export const bar = 2;
```

```query links main.tspp
@links.link range=main.tspp#foo_specifier path=foo.tspp
@links.link range=main.tspp#bar_specifier path=bar.tspp
```

### Resolve the current import target

Links follow the resolved module path after each edit.

```tspp main.tspp
import { value } from "./library.tspp";
                      ^^^^^^^^^^^^^^ specifier
```

```query links main.tspp
@links.none
```

```tspp library.tspp add
export const value = 1;
```

```query links main.tspp
@links.link range=main.tspp#specifier path=library.tspp
```

```move library.tspp moved.tspp
```

```query links main.tspp
@links.none
```

```tspp main.tspp change
import { value } from "./moved.tspp";
                      ^^^^^^^^^^^^ specifier
```

```query links main.tspp
@links.link range=main.tspp#specifier path=moved.tspp
```

## Re-Exports

### Link resolved re-export specifiers

A resolved re-export specifier becomes a link.

```tspp main.tspp
export { foo } from "./foo.tspp";
                    ^^^^^^^^^^ foo_specifier
```

```tspp foo.tspp
export const foo = 1;
```

```query links main.tspp
@links.link range=main.tspp#foo_specifier path=foo.tspp
```

## Side Effects

### Link resolved side-effect imports

A resolved side-effect import becomes a link.

```tspp main.tspp
import "./setup.tspp";
       ^^^^^^^^^^^^ setup_specifier
```

```tspp setup.tspp
export const ready = true;
```

```query links main.tspp
@links.link range=main.tspp#setup_specifier path=setup.tspp
```

## Ordinary Strings

### Ignore ordinary string literals

Ordinary strings are not module links.

```tspp main.tspp
const path = "./foo.tspp";
```

```query links main.tspp
@links.none
```

## Unresolved Imports

### Ignore unresolved import specifiers

An unresolved module has no target location to link.

```tspp main.tspp
import { missing } from "./missing.tspp";
                        ^^^^^^^^^^^^^^ specifier
```

```query links main.tspp
@links.none
```

## Export Stars

### Link an export-star specifier

A resolved export-star specifier becomes a link.

```tspp main.tspp
export * from "./library.tspp";
              ^^^^^^^^^^^^^^ library_specifier
```

```tspp library.tspp
export const value = 1;
```

```query links main.tspp
@links.link range=main.tspp#library_specifier path=library.tspp
```
