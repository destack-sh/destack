# @destack/theme

Theme Destack interfaces with declared themes, palettes grown from seeds, roles solved for contrast and shared design tokens.

## Themes

`defineTheme` declares a package's theme from a base and an accent seed, and `theme.variables` returns the custom properties a theme root carries for an appearance and a person's preferences.

```tsx
import { DEFAULT_PREFERENCES } from "@destack/theme";
import { defineTheme } from "@destack/theme/declare";
import { color, space } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";
import "@destack/theme/theme.css";

export const theme = defineTheme({
    name: "notes",
    base: "sand", // surfaces, text and lines: a preset, or a seed such as "#0b2029"
    accent: "orange", // primary actions, selection, focus and the first chart series
    status: { destructive: "tomato" }, // red, green, amber and blue when absent
    chart: ["teal"], // the series after the accent, the rest picked by hue
    radius: "medium",
    scaling: "100%",
    density: "regular",
    fonts: { text: '"IBM Plex Sans", sans-serif' }, // a replaced text font drops the system font's tracking
    text: { title1: { weight: "medium" } }, // change a text style, such as lighter headings for the font
});

const styles = style.create({
    page: { backgroundColor: color.background, color: color.foreground, padding: space[4] },
});

export function Page() {
    return (
        <main {...style.attrs(styles.page)} style={theme.variables("system", DEFAULT_PREFERENCES)}>
            Hello
        </main>
    );
}
```

## Palettes

`Palette.of` grows the colors of one hue at every OKLCH lightness from a preset or a hex seed, keeping the seed exact at its own lightness and letting its chroma fall toward white and black.

```ts
import { Palette } from "@destack/theme";

const indigo = Palette.of("indigo");
indigo.seed; // "#3e63dd", at lightness 0.544
indigo.tone(0.95); // "#e7eeff"
indigo.tone(0.3); // "#212c4b"
```

## Roles

Each of `ROLE_NAMES` is a tone of a palette, a palette's seed or another role moved in lightness, and a role reading on another moves away from it until it reaches its floor's APCA contrast.

```ts
import { BACKGROUND_LIGHTNESS, CONTENT_CONTRAST } from "@destack/theme";

BACKGROUND_LIGHTNESS; // { light: 0.993, dark: 0.187 }
CONTENT_CONTRAST; // { standard: 60, more: 90 }
```

## Role overrides

`roles` replaces a role with any source the built-in roles use: a palette's tone or seed, another role moved in lightness, or explicit colors, each at an `alpha`.

```ts
defineTheme({
    name: "paper",
    roles: {
        background: { light: "#f8f5ee", dark: "#0b2029" },
        border: { from: "foreground", lightness: { light: 0, dark: 0 }, alpha: 0.2 },
        accent: { palette: "accent", lightness: { light: 0.04, dark: -0.04 }, from: "background" },
    },
});
```

## Translucent roles

A translucent role reading on another settles in opacity: it keeps its declared `alpha` where its blend reaches the floor, and takes the least opacity that does where it does not.

```ts
theme.resolve("light", 0).color("border"); // "#1f202133", a fifth, Lc 23.3
theme.resolve("light", 1).color("border"); // "#1f202141", raised to the line floor, Lc 30
```

## Contrast

A person's contrast level runs from 0, the APCA-W3 bronze floors, to 1, each floor's most, and `system` mixes between the two as far as the device asks for more contrast.

```ts
theme.resolve("light", 0).color("mutedForeground"); // "#62636a", Lc 78.6 on the background
theme.resolve("light", 1).color("mutedForeground"); // "#47474b", Lc 90.1
theme.variables("light", { ...DEFAULT_PREFERENCES, contrast: 0.5 }); // the floors halfway
```

## Contrast checks

Declaring a theme checks every role that reads on another in both appearances at both ends, and refuses one that stays below its floor.

```ts
defineTheme({ name: "fog", roles: { background: { light: "#8b8d98", dark: "#101113" } } });
// PackageError INVALID_DEFINITION: theme fog: foreground on background reads at Lc 43.7
//  in light with standard contrast, below 75
```

## Labels

A label is whichever of white, the darkest text of its background's palette and the base's darkest text reads best on the solid it sits on, and when none reaches its floor the solid moves by the least OKLCH lightness that lets the best one pass.

```ts
const theme = defineTheme({ name: "sunset", accent: "#ff8800" });
theme.resolve("light", 0).color("primary"); // "#ff9946", 0.031 lighter
theme.resolve("light", 0).color("primaryForeground"); // "#271d17", the accent's darkest text
defineTheme({ name: "signal", roles: { primary: { palette: "warning" } } }); // primaryForeground "#231f19" on amber
```

## Chart series

`chart1` is the accent, and the series after it are the declared chart seeds, then the colorful presets whose seeds read as graphics on the light background, each the hue farthest from those taken.

```ts
// the default indigo theme
// chart1 #3e63dd, chart2 #978365 gold, chart3 #29a383 jade, chart4 #d6409f pink, chart5 #00a2c7 cyan
```

## Swatches

Each colorful preset has a swatch, the roles it takes as an accent: a solid fill, the label reading on it, a subtle tint and the text reading on the tint, so an object marked teal reads at every appearance and contrast.

```ts
import { swatch } from "@destack/theme/tokens.stylex";

const tag = style.create({ tag: { backgroundColor: swatch.tealTint, color: swatch.tealText } });
// --destack-swatch-teal-tint: light-dark(#d7fbf3, #1b2826)
```

## Nested surfaces

`rebase` derives a theme over another background, such as a selected row's, so the surfaces above it keep its tint and every text, line and graphic reads against it.

```ts
const row = theme.rebase({ light: "#c9d8ff", dark: "#1d2f66" });
<li style={row.variables("system", preferences)}>…</li>;
// card #c5d4fb, mutedForeground #5b5c62 against the row, #62636a against the page
```

## Tokens

`src/token/tokens.json` holds every token in the W3C Design Tokens format, and `bun run generate` compiles it into StyleX constants whose values are stable `--destack-*` custom properties.

```json
"space": {
    "$description": "Spacing steps, scaled by the scaling and density.",
    "4": { "$type": "dimension", "$value": { "value": 16, "unit": "px" }, "$description": "Spacing step 4." }
}
```

```ts
import {
    color,
    font,
    motion,
    radius,
    shadow,
    size,
    space,
    stroke,
    surface,
    weight,
} from "@destack/theme/tokens.stylex";

space[4]; // "var(--destack-space-4)"
stroke.border; // "var(--destack-stroke-border)", 1px at every scaling
weight.medium; // "var(--destack-weight-medium)", 500
color.mutedForeground; // "var(--destack-color-muted-foreground)"
font.code; // "var(--destack-font-code)", the theme's code font stack
motion.easingSpring; // "var(--destack-motion-easing-spring)"
```

## Text

`text` holds one StyleX style per text style, scaled by the person's text size.

```tsx
import { text } from "@destack/theme/text";

<h1 {...style.attrs(text.largeTitle)}>Notes</h1>;
<p {...style.attrs(text.body)}>Body text</p>;
// at medium: caption 11px, footnote 12, subheadline 14, callout 15, body 16, headline 16,
// title1 27, title2 21, title3 19, largeTitle 33
```

## Motion

`transition` enters elements from `@starting-style` and exits them with `transition-behavior: allow-discrete`, and every duration collapses to zero under reduced motion.

```tsx
import { transition } from "@destack/theme/motion";

<dialog {...style.attrs(isOpen() ? transition.enter : transition.exit)} />;
```

## Element defaults

`theme.css` sets the element defaults that read the theme in the `base` layer after `@destack/style/preflight.css`: page paint and font, code font, form accent, placeholders, selection and the focus ring.

```tsx
import "@destack/style/preflight.css";
import "@destack/theme/theme.css";
```

## Default theme

`destackTheme` is Destack's own theme, which views of packages declaring no theme take.

```ts
import { destackTheme } from "@destack/theme/declare";

destackTheme.variables("system", DEFAULT_PREFERENCES);
```
