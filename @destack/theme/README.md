# @destack/theme

Theme views with an accent, a base and roles solved for contrast, in light and dark.

## Overview

A package declares its theme from an accent and a base, and every view renders it under the person's display settings.

- **Theme**: an accent, a base gray, a radius, a scaling and a density over color roles such as `background`, `primary` and `mutedForeground`; `defineTheme`.
- **Role**: a color the views use, solved per appearance for APCA contrast; `roles` overrides one.
- **Display**: the person's appearance, contrast, text size, density and motion; `variables` turns a theme into CSS custom properties under it.
- **Palette**: OKLCH tones grown from one seed color.
- **Tokens**: the roles, text styles, swatches and transitions as StyleX constants in `@destack/theme/tokens.stylex`.

```ts
import { DEFAULT_DISPLAY } from "@destack/theme";
import { defineTheme } from "@destack/theme/declare";

export const theme = defineTheme({ name: "notes", base: "sand", accent: "orange" });

const light = theme.resolve("light", 1).color("mutedForeground");
const variables = theme.variables({ ...DEFAULT_DISPLAY, appearance: "dark", contrast: 0.5 });
```

## Examples

### Override a role

`roles` replaces a role with explicit colors, or derives it from another role.

```ts
import { defineTheme } from "@destack/theme/declare";

export const paper = defineTheme({
    name: "paper",
    roles: {
        background: { light: "#f8f5ee", dark: "#0b2029" },
        border: { from: "foreground", lightness: { light: 0, dark: 0 }, alpha: 0.2 },
    },
});
```

### Style with theme tokens

A component styles itself with the theme's tokens, which follow the person's theme.

```ts
import * as style from "@destack/style";
import { color, swatch } from "@destack/theme/tokens.stylex";

const styles = style.create({
    tag: { backgroundColor: swatch.tealTint, color: swatch.tealText },
    muted: { color: color.mutedForeground },
});
```

### Theme a nested surface

`rebase` derives a theme over another background, such as a selected row's.

```ts
import { DEFAULT_DISPLAY } from "@destack/theme";
import { destackTheme } from "@destack/theme/declare";

const selected = destackTheme.rebase({ light: "#c9d8ff", dark: "#1d2f66" });
const variables = selected.variables(DEFAULT_DISPLAY);
```

## Comparison

### Radix Themes

`accent`, `base`, `radius` and `scaling` follow Radix Themes' `accentColor`, `grayColor`, `radius` and `scaling`, over shadcn's color roles; three things differ.

- Roles are solved from OKLCH tones for APCA contrast, and a person's `contrast` raises them, where Radix has twelve fixed steps.
- A theme becomes CSS custom properties, where Radix has a `<Theme>` component.
- Themes add a density, and text styles follow Apple's.
