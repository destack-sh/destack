# @platform/brand

`@platform/brand` holds Destack's mark, app icons per channel, favicons and social banners, and `generate` renders their rasters from the SVG masters.

```ts
const mark = import.meta.resolve("@platform/brand/mark.svg");
const icon = import.meta.resolve("@platform/brand/icon/icon-dev-rounded.svg"); // per channel: "", "-dev"
```
