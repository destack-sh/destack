# @destack/theme

Theme Destack interfaces with Radix palettes and shared CSS tokens.

## Themes

`createTheme` returns the attributes and CSS variables to spread onto a theme root, and styles read them through the tokens of `tokens.stylex`.

```tsx
import { createTheme } from "@destack/theme";
import { color, space } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";
import "@destack/theme/theme.css";

const theme = createTheme({
    appearance: "system",
    accent: "indigo",
    gray: "slate",
    radius: "medium",
    scaling: "100%",
});

const styles = style.create({
    page: {
        backgroundColor: color.background,
        color: color.foreground,
        padding: space[4],
    },
});

export function Page() {
    return (
        <main {...style.attrs(styles.page)} {...theme}>
            Hello
        </main>
    );
}
```

## Overrides

`fontFamily` and `monospaceFontFamily` set a theme's font stacks, and any `--destack-` variable in `style` overrides one semantic color.

```ts
import { createTheme } from "@destack/theme";

const publication = createTheme({ fontFamily: '"IBM Plex Sans Variable", sans-serif' });
publication.style["--destack-color-background"] = "light-dark(#f8f7f4, #1b1a19)";
```

## Palette mapping

`createTheme` maps the shadcn color roles to steps of the gray (`neutral`) and accent (`primary`) Radix palettes, and the sidebar roles repeat the card, primary, accent, border and ring steps.

```ts
background: neutral(1),
foreground: neutral(12),
card: neutral(2),
popover: neutral(2),
primary: primary(9),
primaryForeground: paletteForeground(accent),
secondary: neutral(3),
muted: neutral(3),
mutedForeground: neutral(11),
accent: primary(3),
accentForeground: primary(12),
border: neutral(6),
input: neutral(7),
ring: primary(8),
destructive: paletteColor("red", 9),
```

## Appearance

The `appearance` setting selects the system, light or dark appearance for a person, with overrides per package, space, installation or device.

```ts
import { createTheme } from "@destack/theme";
import { appearance } from "@destack/theme/setting";

const theme = createTheme({ appearance: appearance.resolve(selection, rows, chain).value });
```

## License

The package includes palettes from Radix Colors 3.0.0, tokens from Radix Themes and chart colors from shadcn/ui under the MIT License.

```text
Copyright (c) 2021 Radix
Copyright (c) 2023 WorkOS
Copyright (c) 2023 shadcn

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
