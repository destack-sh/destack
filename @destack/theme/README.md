# @destack/theme

Theme views with a page, an accent and ten hues, solved for contrast in light and dark.

## Overview

`@destack/theme/tokens.stylex` is shadcn/ui's neutral roles and ten hues as StyleX constants, and `defineTheme` fills them in with Radix Themes' radius and scaling.

```ts
import { DEFAULT_DISPLAY } from "@destack/theme";
import { defineTheme } from "@destack/theme/declare";

// declare a theme
export const harbour = defineTheme({
    name: "harbour",
    base: "#8d8d86",
    accent: "#c2410c",
    ground: {
        light: { background: "#fbf7f0", foreground: "#2b2118", muted: "#efe7da" },
        dark: { background: "#1c1712", foreground: "#efe6da", muted: "#2b241d" },
    },
});

// compute its custom properties
const variables = harbour.variables({ ...DEFAULT_DISPLAY, appearance: "dark" });
```

## Examples

### Fill in the hues

`hues` sets the colors of tags, statuses and charts, and a hue left out takes Destack's.

```ts
import { defineTheme } from "@destack/theme/declare";

// fill in four hues
export const solar = defineTheme({
    name: "solar",
    base: "#839496",
    accent: "blue", // #268bd2
    hues: { red: "#dc322f", yellow: "#b58900", green: "#859900", blue: "#268bd2" },
});

solar.seed("green"); // #859900
solar.seed("purple"); // #8e4ec6
solar.seed("gray"); // #839496
```

### Style a tag and a status

`swatch` draws each hue and status as a solid fill with its label, a tint with its text, text, and a mark.

```ts
import * as style from "@destack/style";
import { space, swatch } from "@destack/theme/tokens.stylex";

// style by meaning
export const styles = style.create({
    tag: { backgroundColor: swatch.cyanTint, color: swatch.cyanTintText, padding: space[1] },
    remove: { backgroundColor: swatch.destructiveSolid, color: swatch.destructiveLabel }, // red
    warning: { color: swatch.warningText }, // yellow
    line: { stroke: swatch.series2Mark }, // farthest from the accent
});
```

### Style a page

`color` draws the neutral roles, each reading on every ground it sits on.

```ts
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color } from "@destack/theme/tokens.stylex";

// style by role
export const styles = style.create({
    page: { backgroundColor: color.background, color: color.foreground },
    card: { backgroundColor: color.card, borderColor: color.border },
    caption: { color: color.mutedForeground },
});
export const caption = [text.xs, styles.caption];
```

### Display a theme for a person

`Display` sets a person's appearance, sizes, contrast, motion, typeface and accent over a theme.

```ts
import type { Display } from "@destack/theme";
import { titanTheme } from "@destack/theme/declare";

// set a person's display
const display: Display = {
    appearance: "dark",
    textSize: "large",
    density: null, // the theme's
    contrast: "system", // the device's
    motion: "reduced",
    accent: "blue", // the theme's
    typeface: "inter",
};
const variables = titanTheme.variables(display);
```

### Override a role

`roles` replaces a role with explicit colors, or derives it from another role.

```ts
import { defineTheme } from "@destack/theme/declare";

// replace two roles
export const paper = defineTheme({
    name: "paper",
    roles: {
        background: { light: "#f8f5ee", dark: "#0b2029" },
        border: { from: "foreground", lightness: { light: 0, dark: 0 }, alpha: 0.2 }, // a fifth of the ink
    },
});
```

### Tune the type scale

`text` changes a style's size, weight, tracking or font, with fluid sizes growing from a 360px to a 1280px window.

```ts
import { defineTheme } from "@destack/theme/declare";

// tune three styles
export const publication = defineTheme({
    name: "publication",
    text: {
        base: { size: 15 },
        "5xl": { size: { min: 32, max: 48 }, weight: "medium", tracking: -0.02 },
        xs: { font: "code", tracking: 0.1 },
    },
});
```

### Theme a nested surface

`rebase` derives a theme over another background, such as a selected row's.

```ts
import { DEFAULT_DISPLAY } from "@destack/theme";
import { destackTheme } from "@destack/theme/declare";

// derive the theme over a selected row
const selected = destackTheme.rebase({ light: "#c9d8ff", dark: "#1d2f66" });
const variables = selected.variables(DEFAULT_DISPLAY);
```

### Check a theme's contrast

`requireContrast` throws where text reads below its APCA floor or a label below WCAG's 4.5 to 1.

```ts
import { titanTheme } from "@destack/theme/declare";

// check its contrast
titanTheme.requireContrast([0]); // standard
titanTheme.requireContrast(); // every level
```

### Measure a contrast

`Srgb.contrast` measures a color on another by APCA or WCAG, as Color.js does, and `Resolution.contrast` a role.

```ts
import { Srgb } from "@destack/theme";
import { destackTheme } from "@destack/theme/declare";

// measure two colors and a role
Srgb.parse("#ffffff").contrast(Srgb.parse("#c75300"), "wcag");
destackTheme.resolve("dark", 0).contrast("mutedForeground", "apca");
```

## Comparison

### Radix Themes

`defineTheme` takes Radix Themes' `radius` and `scaling`, and replaces its color names with a hue or any color.

```ts
import { defineTheme } from "@destack/theme/declare";

// declare <Theme accentColor="blue" radius="large">
export const theme = defineTheme({
    name: "notes",
    accent: "blue", // accentColor, the theme's own blue
    base: "#8b8d98", // grayColor
    radius: "large",
    scaling: "105%",
    density: "compact", // Radix Themes has none
});
```

### shadcn/ui

`color` has shadcn/ui's neutral variables, and `swatch` its colored ones.

```ts
import * as style from "@destack/style";
import { color, swatch } from "@destack/theme/tokens.stylex";

// style by Destack's tokens
export const styles = style.create({
    card: { backgroundColor: color.card, color: color.foreground }, // bg-card
    primary: { backgroundColor: swatch.accentSolid, color: swatch.accentLabel }, // bg-primary
    destructive: { backgroundColor: swatch.destructiveTint }, // bg-destructive/10
    row: { backgroundColor: { default: null, ":hover": color.hover } }, // hover:bg-accent
    line: { stroke: swatch.series2Mark }, // --chart-2
});
```
