# @destack/theme

Theme views with an accent, a base and roles solved for contrast, in light and dark.

## Overview

`@destack/theme` is Radix Themes' settings over shadcn's neutral roles and one swatch per hue: themes, roles solved for contrast, display settings and StyleX tokens.

Packages declare a theme:

- `Theme`: an accent, a base gray, a radius, a scaling and a density, declared with `defineTheme`.
- `Seed`: a palette's source color: a preset such as `"orange"`, or a hex color.
- `RoleName`: a neutral role such as `background` or `mutedForeground`, or a swatch role such as `solid`.
- `RoleOverride`: a theme's color for a role.
- `Ground`: a theme's page, text and quiet fill at one time of day.
- `THEMES`: Destack's themes, the brand and one per planet.

People see it under their display settings:

- `Display`: a person's appearance, text size, density, contrast, motion and accent.
- `ThemeStyle`: the theme as CSS custom properties.

Components use tokens:

- `@destack/theme/tokens.stylex`: the neutral roles, every hue's swatch, spacing, sizes, radii, shadows and motion as StyleX constants.
- `@destack/theme/mark`: Destack's mark as SVG markup, `MARK`.
- `text` and `transition`: text styles and the overlays' enter and exit transitions.
- `@destack/theme/state`: `state`, the styles of focus, hover, pressed, disabled, selected, checked and other states, the same in every component.

Declare a theme, then compute a view's custom properties in dark mode at more contrast:

```ts
import { DEFAULT_DISPLAY } from "@destack/theme";
import { defineTheme } from "@destack/theme/declare";

// declare a warm theme with an orange accent
export const theme = defineTheme({ name: "notes", base: "sand", accent: "orange" });

// compute the custom properties for dark mode at more contrast
const variables = theme.variables({ ...DEFAULT_DISPLAY, appearance: "dark", contrast: 1 });
```

## Reference

### Theme

`defineTheme(definition)` declares a theme, refusing text below its APCA contrast and labels on fills below WCAG's 4.5 to 1.
`destackTheme` is the theme of packages that declare none.

| Field     | What it sets                                                                                           |
| --------- | ------------------------------------------------------------------------------------------------------ |
| `name`    | the theme's name within the package                                                                    |
| `base`    | the seed of surfaces, text and lines, `"slate"` by default                                             |
| `accent`  | the seed of primary actions, checked controls, focus and the first chart series, `"indigo"` by default |
| `ground`  | the `light` and `dark` `Ground`: page, text and quiet fill as hex colors, which lines follow           |
| `status`  | the seeds of `destructive`, `success`, `warning` and `info`, red, green, amber and blue by default     |
| `series`  | up to four seeds of the chart series after the accent, picked far apart in hue by default              |
| `radius`  | `"none"`, `"small"`, `"medium"`, the default, `"large"` or `"full"`                                    |
| `scaling` | `"90%"`, `"95%"`, `"100%"`, the default, `"105%"` or `"110%"`                                          |
| `density` | `"compact"`, `"regular"`, the default, or `"spacious"`                                                 |
| `fonts`   | the `text` and `code` font stacks, as CSS `font-family` values                                         |
| `text`    | the `size`, `weight`, `tracking` and `font` of text styles, sizes fixed or fluid                       |
| `roles`   | the theme's colors for roles after its ground, by `RoleName`                                           |

A theme answers its colors and custom properties:

| Verb                          | What it does                                                                                       |
| ----------------------------- | -------------------------------------------------------------------------------------------------- |
| `variables(display)`          | computes the custom properties of a view's root, with `color-scheme`                               |
| `resolve(scheme, level, hue)` | resolves the roles in `"light"` or `"dark"` at a contrast from 0 to 1, and `color(name)` reads one |
| `hues(accent)`                | lists the theme's hues by token name: the accent, the statuses and the chart series                |
| `rebase(background)`          | derives the theme over another background, such as a selected row's                                |

### Roles

The neutral roles follow shadcn's names, each solved for its contrast on every surface it reads on.

| Neutral roles                   | What they color                                         |
| ------------------------------- | ------------------------------------------------------- |
| `background`, `card`, `popover` | the page, and raised and floating surfaces              |
| `foreground`, `mutedForeground` | body text and quieter text on every surface             |
| `muted`                         | quiet fills                                             |
| `hover`, `selected`             | hovered and selected items, as layers of the text color |
| `border`, `input`, `ring`       | lines, field borders and focus rings                    |
| `scrim`                         | the shade behind a dialog                               |

Every hue takes the same swatch, after Primer's functional colors: the accent, the statuses, the five chart series and every colorful preset.

| Swatch roles       | What they color                                                   |
| ------------------ | ----------------------------------------------------------------- |
| `solid`, `label`   | a filled control such as a button, and its label                  |
| `tint`, `tintText` | a tinted area such as a tag, and text on it                       |
| `text`             | the hue as text on every surface, such as a link or an error      |
| `mark`             | the hue as a graphic on the page, such as a chart's line or a dot |

A `RoleOverride` takes one of four forms, each with an optional `alpha` but the second.
A palette is `base`, or `hue`, the hue a swatch grows from.
A theme's swatch overrides apply under its own accent alone.

| Override                                        | Color                                                                                       |
| ----------------------------------------------- | ------------------------------------------------------------------------------------------- |
| `{ palette, lightness: { light, dark }, from }` | a palette's tone at an OKLCH lightness in each appearance, offset from a role's with `from` |
| `{ palette }`                                   | a palette's seed                                                                            |
| `{ from, lightness: { light, dark } }`          | another role's color, its lightness offset in each appearance                               |
| `{ light, dark }`                               | a hex color in each appearance                                                              |

### Contrast

Text must reach its APCA lightness contrast Lc, and a label on a fill must also reach WCAG's 4.5 to 1.
APCA judges light text on dark backgrounds correctly, and accessibility audits measure WCAG's ratio.

| Verb                                     | What it does                                                         |
| ---------------------------------------- | -------------------------------------------------------------------- |
| `Srgb.contrast(background, measure)`     | measures a color as text on a background, `"apca"` or `"wcag"`       |
| `Resolution.contrast(name, measure, on)` | measures a resolved role on a role it reads on, the first by default |

`Srgb` is a color in sRGB, the screen's color space, and `Oklch` a color in OKLCH, the space palettes grow in.

```ts
import { Srgb } from "@destack/theme";
import { destackTheme } from "@destack/theme/declare";

// measure white on the brand's solid accent, and the muted text of dark mode
Srgb.parse("#ffffff").contrast(Srgb.parse("#c75300"), "wcag");
destackTheme.resolve("dark", 0).contrast("mutedForeground", "apca");
```

### Display

`DEFAULT_DISPLAY` has each display setting's default.
`@destack/theme/theme.css` sets element defaults and follows the device's contrast and motion.
`@destack/theme/code.css` colors highlighted code under `[data-code]`.

| Field        | What it is                                                                    |
| ------------ | ----------------------------------------------------------------------------- |
| `appearance` | `"system"`, `"light"` or `"dark"`                                             |
| `textSize`   | `"xSmall"` to `"xxxLarge"`, body text from 14 to 23 points, `"medium"` at 16  |
| `density`    | the person's density, or `null` for the theme's                               |
| `contrast`   | `"system"`, following the device's `prefers-contrast`, or a level from 0 to 1 |
| `motion`     | `"system"`, following `prefers-reduced-motion`, `"full"` or `"reduced"`       |
| `accent`     | the person's accent seed, or `null` for the theme's                           |

### Tokens

`@destack/theme/tokens.stylex` has every token as a StyleX constant over its CSS custom property.
`@destack/theme/tokens.json` has the same tokens in the Design Tokens format, with the default theme's values.
`text` and `transition` are StyleX styles, from `@destack/theme/text` and `@destack/theme/motion`.

| Constant     | Tokens                                                                                                                             |
| ------------ | ---------------------------------------------------------------------------------------------------------------------------------- |
| `color`      | every neutral role, such as `color.mutedForeground`                                                                                |
| `swatch`     | every hue's `Solid`, `Label`, `Tint`, `TintText`, `Text` and `Mark`, such as `swatch.accentSolid` or `swatch.series2Mark`          |
| `font`       | `text` and `code`                                                                                                                  |
| `weight`     | `regular`, `medium`, `semibold` and `bold`                                                                                         |
| `space`      | steps `1` to `9`, scaled by scaling and density                                                                                    |
| `size`       | steps `1` to `4`, scaled by scaling and density                                                                                    |
| `width`      | `popover`, `hoverCard`, `sidebar`, `sidebarIcon`, `toast`, `prose`, `tile` and `row`                                               |
| `radius`     | steps `1` to `6`, scaled by the theme's radius, and `full`                                                                         |
| `stroke`     | `border` and `ring`                                                                                                                |
| `shadow`     | `edge`, `cast`, `raised` and `overlay`                                                                                             |
| `motion`     | `durationShort`, `durationMedium`, `durationLong`, `easingStandard`, `easingEmphasised` and `easingSpring`                         |
| `text`       | the text styles after Tailwind's type scale: `xs`, `sm`, `base`, `lg`, `xl` and `2xl` to `9xl`, each a size with its line height   |
| `transition` | `popover`, `modalPopover` and `dialog`, which grow and fade, `sheet`, `notice` and `noticeLeaving`; reduced motion keeps the fades |

## Examples

### Override a role

`roles` replaces a role with explicit colors, or derives it from another role.

```ts
import { defineTheme } from "@destack/theme/declare";

// paint the page in paper and ink
export const paper = defineTheme({
    name: "paper",
    roles: {
        background: { light: "#f8f5ee", dark: "#0b2029" },
        border: { from: "foreground", lightness: { light: 0, dark: 0 }, alpha: 0.2 },
    },
});
```

### Tune the type scale

`text` changes a style's size, weight, tracking or font, with fluid sizes growing from a 360px to a 1280px window.

```ts
import { defineTheme } from "@destack/theme/declare";

// set body text a step smaller, headlines fluid, and labels in the code font
export const publication = defineTheme({
    name: "publication",
    text: {
        base: { size: 15 },
        "5xl": { size: { min: 32, max: 48 }, weight: "medium", tracking: -0.02 },
        xs: { font: "code", tracking: 0.1 },
    },
});
```

### Style with theme tokens

Tokens follow the person's theme and display settings.

```ts
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color, space, swatch } from "@destack/theme/tokens.stylex";

// style a teal tag and a muted caption
export const styles = style.create({
    tag: { backgroundColor: swatch.tealTint, color: swatch.tealTintText, padding: space[1] },
    muted: { color: color.mutedForeground },
});
export const caption = [text.xs, styles.muted];
```

### Theme a nested surface

`rebase` derives a theme over another background, such as a selected row.

```ts
import { DEFAULT_DISPLAY } from "@destack/theme";
import { destackTheme } from "@destack/theme/declare";

// derive the theme over a selected row
const selected = destackTheme.rebase({ light: "#c9d8ff", dark: "#1d2f66" });
const variables = selected.variables(DEFAULT_DISPLAY);
```

## Comparison

### Radix Themes

`accent`, `base`, `radius` and `scaling` follow Radix Themes' `accentColor`, `grayColor`, `radius` and `scaling`, with its color names as presets.
Roles are solved for contrast instead of twelve fixed steps, and a theme is CSS custom properties instead of a `<Theme>` component.
`Srgb.contrast` follows Color.js's `contrast(color, algorithm)`.

| Radix Themes               | `@destack/theme`     | Difference                                                          |
| -------------------------- | -------------------- | ------------------------------------------------------------------- |
| `accentColor`, `grayColor` | `accent`, `base`     | A preset or any hex color.                                          |
| `radius`, `scaling`        | the same             | The same.                                                           |
| `appearance`               | `Display.appearance` | A person's setting, beside text size, density, contrast and motion. |
| the 12-step color scales   | `Palette`            | Tones at any OKLCH lightness, picked for each role's contrast.      |
| none                       | `density`            | Packs controls and content tighter or looser.                       |

### shadcn/ui

Neutral roles follow shadcn/ui's CSS variables; its colored roles become swatches.

| shadcn/ui                              | `@destack/theme`                                    | Difference                                                     |
| -------------------------------------- | --------------------------------------------------- | -------------------------------------------------------------- |
| `--background`, `--card`, `--muted`, … | `--destack-color-background`, …                     | The same neutral roles; text tiers serve every surface.        |
| `--primary`, `--primary-foreground`    | `--destack-swatch-accent-solid`, `…-accent-label`   | One swatch shape for the accent, statuses, series, presets.    |
| `--accent`                             | `--destack-color-hover`, `--destack-color-selected` | State layers of the text color, after Material.                |
| `--chart-1` to `--chart-5`             | `--destack-swatch-series1-mark`, …                  | Full swatches, so charts fill, label and tint from the theme.  |
| hand-picked colors per theme           | roles solved for contrast                           | A theme gives seeds, and each role is solved to read.          |
| `.dark` class                          | `light-dark()` in one value                         | Both appearances in each property, mixed toward more contrast. |
