# Links

## Imports

### Link resolved import specifiers

Each resolved import specifier becomes a link in source order.

```ds main.ds
import { foo } from "./foo.ds";
                    ^^^^^^^^^^ foo_specifier
import { bar } from "./bar.ds";
                    ^^^^^^^^^^ bar_specifier
```

```ds foo.ds
export const foo = 1;
```

```ds bar.ds
export const bar = 2;
```

```query links main.ds
@links.link range=main.ds#foo_specifier path=foo.ds tooltip="Go to ./foo.ds"
@links.link range=main.ds#bar_specifier path=bar.ds tooltip="Go to ./bar.ds"
```

## Re-Exports

### Link resolved re-export specifiers

A resolved re-export specifier becomes a link.

```ds main.ds
export { foo } from "./foo.ds";
                    ^^^^^^^^^^ foo_specifier
```

```ds foo.ds
export const foo = 1;
```

```query links main.ds
@links.link range=main.ds#foo_specifier path=foo.ds tooltip="Go to ./foo.ds"
```

## Side Effects

### Link resolved side-effect imports

A resolved side-effect import becomes a link.

```ds main.ds
import "./setup.ds";
       ^^^^^^^^^^^^ setup_specifier
```

```ds setup.ds
export const ready = true;
```

```query links main.ds
@links.link range=main.ds#setup_specifier path=setup.ds tooltip="Go to ./setup.ds"
```

## Ordinary Strings

### Ignore ordinary string literals

Ordinary strings are not module links.

```ds main.ds
const path = "./foo.ds";
```

```query links main.ds
@links.none
```

## Unresolved Imports

### Ignore unresolved import specifiers

An unresolved module has no target location to link.

```ds main.ds
import { missing } from "./missing.ds";
                        ^^^^^^^^^^^^^^ specifier
```

```query links main.ds
@links.none
```

## Export Stars

### Link an export-star specifier

A resolved export-star specifier becomes a link.

```ds main.ds
export * from "./library.ds";
              ^^^^^^^^^^^^^^ library_specifier
```

```ds library.ds
export const value = 1;
```

```query links main.ds
@links.link range=main.ds#library_specifier path=library.ds tooltip="Go to ./library.ds"
```
