# @destack/style

Style components with StyleX, compiled into one stylesheet per build.

## Overview

Components declare their styles with StyleX, and the build compiles the styles of a package and its dependencies into one stylesheet.

- **Styles**: atomic styles a component declares with `create`, applied with `attrs` as Solid attributes.
- **Variables**: design tokens declared with `defineVars` and overridden with `createTheme`, as the theme package does.
- **media**: Tailwind's breakpoints and pointer queries as StyleX constants, such as `media.maxSm`.
- **styleExtension**: the build extension compiling every package's styles into `virtual:stylex.css`.

```ts
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";

const styles = style.create({
    row: { display: "flex", gap: { default: "1rem", [media.maxSm]: "0.5rem" } },
    muted: { opacity: 0.6 },
});

const attributes = style.attrs(styles.row, styles.muted);
```

## Examples

### Add an inline style

`attributes` applies styles, then an inline style for values known only at run time.

```ts
import * as style from "@destack/style";

const styles = style.create({ bar: { height: "0.5rem" } });

const progress = (ratio: number) => style.attributes(styles.bar, { width: `${ratio * 100}%` });
```

### Reset browser styles

`preflight.css` sits in a `base` layer below StyleX's priority layers, so every component style wins.

```css
@import "@destack/style/preflight.css";
@layer base, priority1, priority2;
```

## Comparison

### StyleX

Styles, variables and themes are StyleX's, which the package wraps; three things differ.

- `attrs` returns attributes typed for Solid.
- `media` adds Tailwind's breakpoints and pointer queries, and `preflight.css` adds Tailwind's preflight.
- One build extension compiles the styles of a package and every dependency into one stylesheet.
