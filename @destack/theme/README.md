# @destack/theme

`defineTheme` is Radix Themes' theme (`accent` its `accentColor`, `base` its `grayColor`, `radius`, `scaling`) over shadcn's color roles (`background`, `foreground`, `primary`, `mutedForeground`, `chart1`…), with Apple's text styles in `@destack/theme/text` and W3C design tokens as StyleX constants in `@destack/theme/tokens.stylex`.

```ts
defineTheme({ name: "notes", base: "sand", accent: "orange", density: "compact" }); // Radix Themes has no density
Palette.of("indigo").tone(0.95); // OKLCH tones from a seed, where Radix has twelve steps
theme.resolve("light", 1).color("mutedForeground"); // roles solved for APCA contrast
theme.variables("system", preferences); // custom properties, where Radix has a <Theme> component
```

## Contrast

`contrast` in a person's preferences runs from 0, the APCA bronze floors, to 1, and `defineTheme` refuses a role below its floor.

```ts
theme.variables("light", { ...DEFAULT_PREFERENCES, contrast: 0.5 });
```

## Role overrides

`roles` replaces a role with a palette tone, another role moved in lightness, or explicit colors.

```ts
defineTheme({
    name: "paper",
    roles: {
        background: { light: "#f8f5ee", dark: "#0b2029" },
        border: { from: "foreground", lightness: { light: 0, dark: 0 }, alpha: 0.2 },
    },
});
```

## Nested surfaces

`rebase` derives a theme over another background, such as a selected row's.

```tsx
const row = theme.rebase({ light: "#c9d8ff", dark: "#1d2f66" });

<li style={row.variables("system", preferences)}>Groceries</li>;
```

## Swatches

`swatch` holds each preset's solid, label, tint and tint text for marking objects.

```ts
const styles = style.create({ tag: { backgroundColor: swatch.tealTint, color: swatch.tealText } });
```

## Motion

`transition` enters elements from `@starting-style` and exits them with discrete transitions.

```tsx
<dialog {...style.attrs(isOpen() ? transition.enter : transition.exit)} />
```

## Element defaults

`theme.css` sets the element defaults that read the theme, in `@destack/style`'s `base` layer.

```ts
import "@destack/theme/theme.css";
```

## Default theme

`destackTheme` is the theme of views whose packages declare none.

```ts
destackTheme.variables("system", DEFAULT_PREFERENCES);
```
