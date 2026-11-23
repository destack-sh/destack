# @destack/color

Name the hues colors mean things in, and the preset palettes accents grow from.

## Overview

`@destack/color` is Notion's tag colors with cyan, and [Radix Colors](https://www.radix-ui.com/colors)' names and seeds:

- `Hue`: a color by its meaning, such as a tag's or a package's, which each theme fills in.
- `Preset`: a Radix palette name, for a person's accent or a theme's base.

## Examples

### Pick a hue

`Hue` validates the hue a declaration stores, and `DEFAULT_HUES` has Destack's seed of each.

```ts
import { Hue, DEFAULT_HUES } from "@destack/color";

// validate the hue a package declares, and read the seed it has where no theme fills it in
const hue = Hue.parse("cyan");
const seed = DEFAULT_HUES[hue];
```

### Offer accents

`ACCENT_PRESETS` lists the colorful presets, and `PRESETS` has the seed of each.

```ts
import { ACCENT_PRESETS, PRESETS } from "@destack/color";

// list the accents a person chooses from, each with the color it grows from
const accents = ACCENT_PRESETS.map((name) => ({ name, seed: PRESETS[name] }));
```
