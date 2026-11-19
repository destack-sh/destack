# @destack/color

Name the preset color palettes and the seed each one grows from.

## Overview

`@destack/color` is the palette vocabulary of [Radix Colors](https://www.radix-ui.com/colors): the names a theme, a package or an object type picks a color by.
Themes grow each preset's scale from its seed; declarations store the name alone.

## Reference

| Export | What it is |
|---|---|
| `PRESET_NAMES` | The preset palette names, neutral grays first. |
| `PRESETS` | The seed each preset palette grows from. |
| `GRAY_PRESETS` | The neutral presets, for a theme's base. |
| `ACCENT_PRESETS` | The colorful presets, which accents, chart series and swatches take. |
| `Preset` | A preset palette name. |
| `SwatchName` | A colorful preset a declaration picks, such as a package's color. |

## Example

Validate a declared color and read its seed:

```ts
import { PRESETS, SwatchName } from "@destack/color";

// validate the color a package declares and read the seed its scale grows from
const color = SwatchName.parse("teal");
const seed = PRESETS[color];
```
