# @destack/theme

Theme views with an accent, a base and roles solved for contrast, in light and dark.

## Overview

`@destack/theme` is Radix Themes' settings over shadcn's color roles: themes, roles solved for APCA contrast, display settings and StyleX tokens.

Packages declare a theme:

- `Theme`: an accent, a base gray, a radius, a scaling and a density, declared with `defineTheme`.
- `Seed`: a palette's source color: a preset such as `"orange"`, or a hex color.
- `RoleName`: a color the views use, such as `background`, `primary` or `mutedForeground`.
- `RoleOverride`: your theme's own color for a role.

People see it under their display settings:

- `Display`: a person's appearance, text size, density, contrast, motion and accent.
- `ThemeStyle`: the theme as CSS custom properties.

Components use tokens:

- `@destack/theme/tokens.stylex`: the roles, surfaces, swatches, spacing, sizes, radii, shadows and motion as StyleX constants.
- `text` and `transition`: text styles and enter and exit transitions.

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

`defineTheme(definition)` declares a theme, refusing text roles below their contrast.
`destackTheme` is the theme of packages that declare none.

| Field     | What it sets                                                                                       |
| --------- | -------------------------------------------------------------------------------------------------- |
| `name`    | the theme's name within the package                                                                |
| `base`    | the seed of surfaces, text and lines, `"slate"` by default                                         |
| `accent`  | the seed of primary actions, selection, focus and the first chart series, `"indigo"` by default    |
| `status`  | the seeds of `destructive`, `success`, `warning` and `info`, red, green, amber and blue by default |
| `chart`   | up to four seeds of the chart series after the accent, picked far apart in hue by default          |
| `radius`  | `"none"`, `"small"`, `"medium"`, the default, `"large"` or `"full"`                                |
| `scaling` | `"90%"`, `"95%"`, `"100%"`, the default, `"105%"` or `"110%"`                                      |
| `density` | `"compact"`, `"regular"`, the default, or `"spacious"`                                             |
| `fonts`   | the `text` and `code` font stacks, as CSS `font-family` values                                     |
| `text`    | the `weight` of text styles, such as lighter headings for a font                                   |
| `roles`   | your own colors for roles, by `RoleName`                                                           |

A theme answers its colors and custom properties:

| Verb                             | What it does                                                                                       |
| -------------------------------- | -------------------------------------------------------------------------------------------------- |
| `variables(display)`             | computes the custom properties of a view's root, with `color-scheme`                               |
| `resolve(scheme, level, accent)` | resolves the roles in `"light"` or `"dark"` at a contrast from 0 to 1, and `color(name)` reads one |
| `rebase(background)`             | derives the theme over another background, such as a selected row's                                |

### Roles

The roles follow shadcn's, each solved for its contrast.

| Roles                                                                     | What they color                                              |
| ------------------------------------------------------------------------- | ------------------------------------------------------------ |
| `background`, `foreground`                                                | the page, and body text on it                                |
| `card`, `cardForeground`, `popover`, `popoverForeground`                  | raised and floating surfaces, and text on them               |
| `primary`, `primaryForeground`                                            | primary actions in the accent, and their labels              |
| `secondary`, `secondaryForeground`, `muted`, `mutedForeground`            | quiet fills, and quieter text                                |
| `accent`, `accentForeground`                                              | a tint of the accent, such as a hovered item, and text on it |
| `destructive`, `success`, `warning`, `info`, and each one's `…Foreground` | statuses, and their labels                                   |
| `border`, `input`, `ring`                                                 | lines, field borders and focus rings                         |
| `scrim`                                                                   | the shade behind a dialog                                    |
| `chart1` to `chart5`                                                      | chart series, the accent first                               |

A `RoleOverride` takes one of four forms, each with an optional `alpha` but the second.
A palette is `base`, `accent`, `destructive`, `success`, `warning`, `info` or `chart2` to `chart5`.

| Override                                        | Color                                                                                       |
| ----------------------------------------------- | ------------------------------------------------------------------------------------------- |
| `{ palette, lightness: { light, dark }, from }` | a palette's tone at an OKLCH lightness in each appearance, offset from a role's with `from` |
| `{ palette }`                                   | a palette's seed                                                                            |
| `{ from, lightness: { light, dark } }`          | another role's color, its lightness offset in each appearance                               |
| `{ light, dark }`                               | a hex color in each appearance                                                              |

### Display

`DEFAULT_DISPLAY` has each display setting's default.
`@destack/theme/theme.css` sets element defaults and follows the device's contrast and motion.

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
`text` and `transition` are styles of their own, from `@destack/theme/text` and `@destack/theme/motion`.

| Constant     | Tokens                                                                                                                                            |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| `color`      | every role, such as `color.mutedForeground`                                                                                                       |
| `surface`    | `base`, `raised` and `overlay`, the background, card and popover                                                                                  |
| `swatch`     | each preset's `Solid`, `Label`, `Tint` and `Text`, such as `swatch.tealTint`                                                                      |
| `font`       | `text` and `code`                                                                                                                                 |
| `weight`     | `regular`, `medium`, `semibold` and `bold`                                                                                                        |
| `space`      | steps `1` to `9`, scaled by scaling and density                                                                                                   |
| `size`       | steps `1` to `4`, scaled by scaling and density                                                                                                   |
| `width`      | `popover`, `hoverCard`, `sidebar`, `sidebarIcon`, `toast`, `prose`, `tile` and `row`                                                              |
| `radius`     | steps `1` to `6`, scaled by the theme's radius, and `full`                                                                                        |
| `stroke`     | `border` and `ring`                                                                                                                               |
| `shadow`     | `edge`, `cast`, `inset`, `raised` and `overlay`                                                                                                   |
| `motion`     | `durationShort`, `durationMedium`, `durationLong`, `easingStandard`, `easingEmphasised` and `easingSpring`                                        |
| `text`       | the text styles after Apple's: `caption`, `footnote`, `body`, `callout`, `subheadline`, `headline`, `title1`, `title2`, `title3` and `largeTitle` |
| `transition` | `enter` and `exit`, which scale with the person's motion                                                                                          |

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

### Style with theme tokens

Tokens follow the person's theme and display settings.

```ts
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color, space, swatch } from "@destack/theme/tokens.stylex";

// style a teal tag and a muted caption
export const styles = style.create({
    tag: { backgroundColor: swatch.tealTint, color: swatch.tealText, padding: space[1] },
    muted: { color: color.mutedForeground },
});
export const caption = [text.caption, styles.muted];
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

`accent`, `base`, `radius` and `scaling` basically follow Radix Themes' `accentColor`, `grayColor`, `radius` and `scaling`, with its color names as presets.
Roles are solved for APCA contrast instead of twelve fixed steps, and a theme is CSS custom properties instead of a `<Theme>` component.

| Radix Themes               | `@destack/theme`     | Difference                                                          |
| -------------------------- | -------------------- | ------------------------------------------------------------------- |
| `accentColor`, `grayColor` | `accent`, `base`     | A preset or any hex color.                                          |
| `radius`, `scaling`        | the same             | The same.                                                           |
| `appearance`               | `Display.appearance` | A person's setting, beside text size, density, contrast and motion. |
| the 12-step color scales   | `Palette`            | Tones at any OKLCH lightness, picked for each role's contrast.      |
| none                       | `density`            | Packs controls and content tighter or looser.                       |

### shadcn/ui

Color roles follow shadcn/ui's CSS variables, and charts its `chart-1` to `chart-5`.

| shadcn/ui                      | `@destack/theme`                | Difference                                                     |
| ------------------------------ | ------------------------------- | -------------------------------------------------------------- |
| `--background`, `--primary`, … | `--destack-color-background`, … | The same roles, plus `success`, `warning`, `info` and `scrim`. |
| hand-picked colors per theme   | roles solved for contrast       | A theme gives seeds, and each role is solved to read.          |
| `.dark` class                  | `light-dark()` in one value     | Both appearances in each property, mixed toward more contrast. |
