# @destack/theme

Theme Destack interfaces with declared themes, generated color scales and shared design tokens.

## Themes

`defineTheme` declares a package's theme, and `theme.variables` returns the custom properties a theme root carries for an appearance and a person's preferences.

```tsx
import { DEFAULT_PREFERENCES } from "@destack/theme";
import { defineTheme } from "@destack/theme/declare";
import { color, space } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";
import "@destack/theme/theme.css";

export const theme = defineTheme({
    name: "notes",
    accent: "orange", // a preset, or a seed such as "#7c3aed"
    gray: "sand",
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

## Contrast

Declaring a theme measures every text role on its background with APCA in both appearances and both contrasts, and refuses a pair below Lc 75 for body text or Lc 60 for other text.

```ts
defineTheme({ name: "fog", roles: { background: { light: "#8b8d98", dark: "#111113" } } });
// PackageError INVALID_DEFINITION: theme fog: foreground on background reads at Lc 41.0
//  in light with standard contrast, below 75
```

## Labels

A label is whichever of white, its scale's step 12 and the gray's step 12 reads best on the role it sits on, and when none reaches Lc 60 on a solid step 9 the scale shifts step 9's OKLCH lightness by the least amount that lets the best one pass.

```ts
const theme = defineTheme({ name: "sunset", accent: "#ff8800" });
theme.scales.accent.color(9, "light"); // "#ff9946", 0.031 lighter, labelled with the gray's step 12 at Lc 60
theme.scales.accent.color(10, "light"); // the hover step, derived from the shifted step 9
defineTheme({ name: "signal", roles: { primary: { scale: "amber", step: 9 } } }); // primaryForeground "#1c2024" on amber
```

## Tokens

`src/token/tokens.json` holds every token in the W3C Design Tokens format, and `bun run generate` compiles it into StyleX constants whose values are stable `--destack-*` custom properties.

```json
"space": {
    "$description": "Spacing steps after Radix Themes, scaled by the scaling and density.",
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

## Roles

Color roles take shadcn/ui's names plus surface levels and status roles, each a step of the theme's gray, accent or a status scale.

```text
background, card, popover   gray 1, 2, 1 (dark 3)    = surface.base, surface.raised, surface.overlay
foreground, *Foreground     gray 12                  on its background, Lc 75
mutedForeground             gray 11 (more: 12)       on background, Lc 60
primary, accent, ring       accent 9, 3, 8
primaryForeground           white, accent 12, gray 12 whichever reads best on primary, Lc 60
destructive, success,       red 9, green 9,
warning, info               amber 9, blue 9          each with a Foreground label read the same way
border, input               gray 6, 7 (more: 8)
scrim                       gray 12 (dark 1) at 50% opacity, behind dialogs, sheets and drawers
```

## Role overrides

`roles` replaces any color role with a scale step or explicit light and dark colors, and the replacement is checked like every other role.

```ts
defineTheme({
    name: "paper",
    roles: {
        background: { light: "#f8f5ee", dark: "#0b2029" },
        mutedForeground: { scale: "gray", step: 12 },
    },
});
```

## Scales

`Scale.preset` reads the Radix Colors scales, and `Scale.generate` builds the same twelve steps in OKLCH around a seed that stays step 9 in both appearances.

```ts
import { apca, Scale } from "@destack/theme";

const indigo = Scale.preset("indigo");
const violet = Scale.generate("#7c3aed");
violet.color(3, "dark"); // the dark component background step
apca("#888888", "#ffffff"); // 63.06, dark text on a light background
```

## Text

`text` holds one StyleX style per text style, named like Apple's at Apple's Medium sizes and scaled by the person's text size.

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
const style = theme.variables(appearance, preferences); // a person's accent replaces the app's own
```

## Default theme

`destackTheme` is Destack's own theme, which views of packages declaring no theme take.

```ts
import { destackTheme } from "@destack/theme/declare";

destackTheme.variables("system", DEFAULT_PREFERENCES);
```

## License

The package includes palettes from Radix Colors 3.0.0 and tokens from Radix Themes under the MIT License.

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
