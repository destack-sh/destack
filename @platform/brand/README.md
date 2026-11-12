# @platform/brand

Draw Destack's brand: its mark, wordmarks, app icons, favicons and social banners.

## Overview

The brand package keeps every asset once, its SVG masters authored and its rasters rendered from them.

- **Mark**: the Destack mark, `mark.svg`.
- **Wordmark**: the name in Destack's type, light and dark, in `wordmark/`.
- **Icon**: the app icon per channel, the release's and the local stage's, square and rounded, and the macOS `.icns` the release builds once, in `icon/`.
- **Banner**: the social banners for GitHub, LinkedIn, X and Open Graph, in `banner/`.

```ts
const mark = import.meta.resolve("@platform/brand/mark.svg");
const icon = import.meta.resolve("@platform/brand/icon/icon-local-rounded.svg");
```

## Examples

### Render the rasters

`generate` renders the icons, favicons and banners at their sizes from the SVG masters.

```sh
bun run generate
```
