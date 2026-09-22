Use Radix palettes and shared CSS tokens in Destack.

## Usage

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
    return <main {...style.attrs(styles.page)} {...theme}>Hello</main>;
}
```

## Overrides

Set application fonts or override semantic CSS variables:

```ts
import { createTheme } from "@destack/theme";

const publication = createTheme({ fontFamily: '"IBM Plex Sans Variable", sans-serif' });
publication.style["--destack-color-background"] = "light-dark(#f8f7f4, #1b1a19)";
```

## Palette Mapping

Destack maps shadcn roles to Radix scales as follows:

| Roles                                 | Radix steps                 |
| ------------------------------------- | --------------------------- |
| Background / foreground               | Gray 1 / 12                 |
| Card, popover, sidebar / foreground   | Gray 2 / 12                 |
| Secondary / foreground                | Gray 3 / 12                 |
| Muted / foreground                    | Gray 3 / 11                 |
| Primary, sidebar primary / foreground | Accent 9 / contrasting text |
| Accent, sidebar accent / foreground   | Accent 3 / 12               |
| Border, sidebar border                | Gray 6                      |
| Input                                 | Gray 7                      |
| Ring, sidebar ring                    | Accent 8                    |
| Destructive                           | Red 9                       |

This mapping adapts the two systems; it is not an upstream Radix or shadcn preset.

## License

Includes palettes from Radix Colors 3.0.0, tokens from Radix Themes, and chart colors from
shadcn/ui, under the MIT License.

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
