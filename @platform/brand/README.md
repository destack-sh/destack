# @platform/brand

Draw Destack's brand: its mark, wordmarks, app icons, favicons and social banners.

## Overview

`@platform/brand` is Destack's brand assets: SVG masters, and the rasters rendered from them.
The mark itself lives in `@destack/theme`, which published packages depend on.

It has:

- `wordmark/`: the name in Destack's type, light and dark.
- `icon/`: the app icon per channel, square and rounded, and the macOS `.icns`.
- `banner/`: the social banners for GitHub, LinkedIn, X and Open Graph.

Resolve an asset's file, and render the rasters from the masters:

```ts
// resolve the local stage's rounded icon
export const icon = import.meta.resolve("@platform/brand/icon/icon-local-rounded.svg");
```

```sh
bun run generate
```
