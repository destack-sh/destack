# @platform/brand

Draw Destack's brand: its mark, wordmarks, app icons, favicons and social banners.

## Overview

`@platform/brand` is Destack's brand assets: SVG masters, and the rasters rendered from them.

It has:

- `mark.svg`: the Destack mark.
- `wordmark/`: the name in Destack's type, light and dark.
- `icon/`: the app icon per channel, square and rounded, and the macOS `.icns`.
- `banner/`: the social banners for GitHub, LinkedIn, X and Open Graph.

Resolve an asset's file, and render the rasters from the masters:

```ts
// resolve the mark and the local stage's rounded icon
export const mark = import.meta.resolve("@platform/brand/mark.svg");
export const icon = import.meta.resolve("@platform/brand/icon/icon-local-rounded.svg");
```

```sh
bun run generate
```
