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
    fonts: { text: '"IBM Plex Sans", sans-serif' },
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

Each color role is a tone of a palette, a palette's seed or another role's color moved in lightness, and a role that reads on another starts from its lightness and moves away from that background until it reaches its APCA contrast.

```text
background                 base at lightness 0.993, dark 0.187
card, popover              the background moved by -0.011 and 0, dark +0.026 and +0.073
secondary, muted           the background moved by -0.034, dark +0.077
foreground, *Foreground    base from 0.242, dark 0.948      on its surface, Lc 75 to 90
mutedForeground            base from 0.502, dark 0.768      on background, Lc 60 to 90
accent, accentForeground   accent at the subtle offset, its text from 0.335, dark 0.915
primary                    the accent's seed                its label at Lc 60 to 75
destructive, success,      the status seeds                 each with a label read the same way
warning, info
border, input              base from 0.886 and 0.852, dark 0.348 and 0.4    on background, Lc 0 to 30
ring                       accent from 0.736, dark 0.532    on background, Lc 45 to 60
scrim                      base at 0.242, dark black, at 50% opacity
chart1 to chart5           the accent's and four series seeds               on background, Lc 45 to 60
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

## Nested surfaces

`rebase` derives a theme over another background, such as a selected row's, so the surfaces above it keep its tint and every text, line and graphic reads against it.

```ts
const row = theme.rebase({ light: "#c9d8ff", dark: "#1d2f66" });
<li style={row.variables("system", preferences)}>…</li>;
// card #c5d4fb, mutedForeground #5b5c62 against the row, #62636a against the page
```

## Role overrides

`roles` replaces any color role with a palette's seed or tone or explicit light and dark colors, and the replacement is checked like every other role.

```ts
defineTheme({
    name: "paper",
    roles: {
        background: { light: "#f8f5ee", dark: "#0b2029" },
        mutedForeground: { palette: "base", lightness: { light: 0.242, dark: 0.948 } },
    },
});
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
    motion,
    radius,
    shadow,
    size,
    space,
    stroke,
    surface,
    text,
    weight,
} from "@destack/theme/tokens.stylex";

space[4]; // "var(--destack-space-4)"
stroke.border; // "var(--destack-stroke-border)", 1px at every scaling
weight.medium; // "var(--destack-weight-medium)", 500
color.mutedForeground; // "var(--destack-color-muted-foreground)"
motion.easingSpring; // "var(--destack-motion-easing-spring)"
```

## Text

`text` holds one StyleX style per text style, scaled by the person's text size.

```tsx
import { text } from "@destack/theme/text";

<h1 {...style.attrs(text.largeTitle)}>Notes</h1>;
<p {...style.attrs(text.body)}>Body text</p>;
// at medium: caption 11px, footnote 12, body 16, callout 15, headline 16,
// title1 27, title2 21, title3 19, largeTitle 33
```

## Motion

`transition` enters elements from `@starting-style` and exits them with `transition-behavior: allow-discrete`, and every duration collapses to zero under reduced motion.

```tsx
import { transition } from "@destack/theme/motion";

<dialog {...style.attrs(isOpen() ? transition.enter : transition.exit)} />;
```

## Preferences

The `appearance`, `textSize`, `density`, `contrast`, `motion` and `accent` settings hold a person's display preferences, and `resolveDisplay` resolves them all for a selection.

```ts
import { DISPLAY_SETTINGS, resolveDisplay } from "@destack/theme/setting";

// the values placed for DISPLAY_SETTINGS along the person's scope chain, nearest first
const { appearance, preferences } = resolveDisplay(
    { scope: person, package: packageId, space, installation },
    values,
    chain,
);
const style = theme.variables(appearance, preferences); // a person's accent replaces the app's own and its chart series
```

## Default theme

`destackTheme` is Destack's own theme, which views of packages declaring no theme take.

```ts
import { destackTheme } from "@destack/theme/declare";

destackTheme.variables("system", DEFAULT_PREFERENCES);
```

## License

The preset seeds derive from Radix Colors 3.0.0 and some token values from Radix Themes, under the MIT License.

```text
Copyright (c) 2021 Radix
Copyright (c) 2023 WorkOS

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```
