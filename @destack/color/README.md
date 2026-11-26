# @destack/color

Read, change and write colors, and name the hues they mean things in.

## Overview

`@destack/color` is a color in sRGB, HSL, HSB or OKLCH, and ten hues in Radix Colors' values, which a theme may replace.

```ts
import { Color, DEFAULT_HUES, Hue } from "@destack/color";

// read a hue's color without a theme
const hue = Hue.parse("cyan");
const color = DEFAULT_HUES[hue]; // #00a2c7

// make it lighter as people see it
const lighter = Color.parse(color).to("oklch").with("lightness", 80).toString();

// name the hue in the person's language, as a picker offers it
const { title } = Hue.choices[hue]; // the message "Cyan", "Ciano" in Italian
```
