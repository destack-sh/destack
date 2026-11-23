# @destack/color

Name the hues colors mean things in, which each theme fills in.

## Overview

`@destack/color` is ten hues in Radix Colors' values, which a theme may replace.

```ts
import { DEFAULT_HUES, Hue } from "@destack/color";

// read a hue's color without a theme
const hue = Hue.parse("cyan");
const color = DEFAULT_HUES[hue]; // #00a2c7
```
