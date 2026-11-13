# @destack/style

Style components with StyleX, compiled into one stylesheet per build.

## Overview

`@destack/style` is StyleX with Tailwind's media queries and preflight: atomic styles, shared conditions, and one stylesheet per view.

Components declare and apply styles:

- `create`: declares a component's styles, as in StyleX.
- `attrs` and `attributes`: apply styles as `class` and `style` attributes.
- `defineVars` and `createTheme`: declare CSS custom properties, and override them under an element.

Shared conditions and resets:

- `media`: Tailwind's breakpoints and media variants as StyleX constants, such as `media.maxSm`.
- `frame`: a page's columns, margin, gutter and widest width at each breakpoint, as StyleX variables.
- `preflight.css`: Tailwind's preflight, below every component style.

Builds compile the styles:

- `styleExtension`: compiles each package's styles.

Declare a row that tightens on small screens, and apply it with a muted style:

```ts
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";

// declare a row that tightens on small screens, and a muted style
const styles = style.create({
    row: { display: "flex", gap: { default: "1rem", [media.maxSm]: "0.5rem" } },
    muted: { opacity: 0.6 },
});

// apply both to an element
const attributes = style.attrs(styles.row, styles.muted);
```

## Reference

### Styles

The style verbs are StyleX's, plus `attrs` and `attributes` for Solid and plain HTML.

| Verb                                              | What it does                                                                              |
| ------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| `create(styles)`                                  | declares named styles, compiled into atomic classes                                       |
| `attrs(...styles)`                                | answers `{ class, style }` for styles, later ones winning                                 |
| `attributes(styles, inline)`                      | answers `{ class, style }` with an inline style after StyleX's                            |
| `props(...styles)`                                | answers `{ className, style }`, as React takes them                                       |
| `firstThatWorks(...values)`                       | uses the first value the browser supports                                                 |
| `defineVars(tokens)`, `defineConsts(values)`      | declares custom properties, or constants inlined at build time                            |
| `createTheme(variables, values)`                  | overrides custom properties under an element                                              |
| `types`                                           | types a variable, such as `types.color(…)`, so it can animate                             |
| `when`, `defineMarker`, `defaultMarker`           | style an element by an ancestor's or sibling's state                                      |
| `keyframes`, `positionTry`, `viewTransitionClass` | declare animations, anchor position fallbacks and view transitions                        |
| `env`                                             | reads a value the StyleX compiler is given                                                |
| `Styles`                                          | the type `attrs` takes: compiled styles, falsy values for skipped ones, and nested arrays |

### media

`media` from `@destack/style/media.stylex` has Tailwind's breakpoints and media variants.

| Constant                                     | Query                                            |
| -------------------------------------------- | ------------------------------------------------ |
| `sm`, `md`, `lg`, `xl`, `2xl`                | a screen at least 40, 48, 64, 80 or 96 rem wide  |
| `maxSm`, `maxMd`, `maxLg`, `maxXl`, `max2xl` | a screen narrower than 40, 48, 64, 80 or 96 rem  |
| `hover`                                      | a primary pointer that hovers                    |
| `pointerCoarse`                              | a coarse primary pointer, such as a finger       |
| `motionReduce`, `motionSafe`                 | a person who asked for less motion, or allows it |
| `contrastMore`                               | a person who asked for more contrast             |
| `print`                                      | a printed page                                   |
| `portrait`, `landscape`                      | a screen held upright or sideways                |

### frame

`frame` from `@destack/style/frame.stylex` has a page's layout grid: Material's columns, margins and gutters on Tailwind's breakpoints.
A page lays its grid out from the variables, and sets its own with `createTheme`.

| Variable  | Value                                 |
| --------- | ------------------------------------- |
| `columns` | 4 columns, 8 from `sm`, 12 from `lg`  |
| `margin`  | 16px beside the frame, 24px from `sm` |
| `gutter`  | 16px between columns, 24px from `sm`  |
| `max`     | `80rem`, the widest the frame grows   |

```ts
import * as style from "@destack/style";
import { frame } from "@destack/style/frame.stylex";

// lay a page out on the frame's columns
export const styles = style.create({
    page: {
        columnGap: frame.gutter,
        display: "grid",
        gridTemplateColumns: `repeat(${frame.columns}, minmax(0, 1fr))`,
        marginInline: "auto",
        width: `min(${frame.max}, 100% - 2 * ${frame.margin})`,
    },
});
```

### Stylesheet

`styleExtension` compiles the styles of every package that depends on `@destack/style`.
Importing `virtual:stylex.css` gives one stylesheet with the rules of a build and its dependencies.

| Layer               | What it holds                                    |
| ------------------- | ------------------------------------------------ |
| `base`              | `preflight.css` and the theme's element defaults |
| StyleX's own layers | component styles, which win over `base`          |

## Examples

### Set a width at run time

`attributes` adds an inline style for a value known only at run time.

```ts
import * as style from "@destack/style";

// declare the bar's fixed height
const styles = style.create({ bar: { height: "0.5rem", backgroundColor: "currentColor" } });

/** Answer a progress bar's attributes at a ratio from 0 to 1. */
export function progressBar(ratio: number) {
    return style.attributes(styles.bar, { width: `${ratio * 100}%` });
}
```

### Override variables under an element

`defineVars` declares custom properties with defaults, and `createTheme` gives them other values under an element.

```ts
import * as style from "@destack/style";

// declare a card's colors, and a theme for a highlighted card
export const card = style.defineVars({ fill: "white", ink: "black" });
const highlighted = style.createTheme(card, { fill: "lightyellow" });

// style the card with its variables
const styles = style.create({ card: { backgroundColor: card.fill, color: card.ink } });

/** Answer a card's attributes, highlighted or not. */
export function cardAttributes(isHighlighted: boolean) {
    return style.attrs(isHighlighted && highlighted, styles.card);
}
```

### Hover only with a pointer that hovers

Put `:hover` inside `media.hover`, so a tap leaves no hover behind.

```ts
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";

// tint the row on hover, on devices that hover
export const styles = style.create({
    row: {
        backgroundColor: {
            default: "transparent",
            [media.hover]: { default: "transparent", ":hover": "rgb(0 0 0 / 0.05)" },
        },
    },
});
```

## Comparison

### StyleX

Styles, variables and themes are StyleX's, exported from here.

| StyleX                                    | `@destack/style`           | Difference                                                         |
| ----------------------------------------- | -------------------------- | ------------------------------------------------------------------ |
| `stylex.create`, `props`, `defineVars`, … | `style.create`, `props`, … | The same.                                                          |
| `stylex.attrs`                            | `attrs`, `attributes`      | `attributes` adds an inline style after StyleX's own.              |
| the bundler plugin's stylesheet           | `virtual:stylex.css`       | Gathers the rules of every dependency's build into one stylesheet. |

### Tailwind

Media constants and the reset follow Tailwind v4.

| Tailwind                                | `@destack/style`                              | Difference                                  |
| --------------------------------------- | --------------------------------------------- | ------------------------------------------- |
| `sm:`, `max-sm:`, …                     | `media.sm`, `media.maxSm`, …                  | The same breakpoints, as StyleX conditions. |
| `motion-reduce:`, `contrast-more:`, …   | `media.motionReduce`, `media.contrastMore`, … | The same queries.                           |
| `hover:` inside `@media (hover: hover)` | `media.hover`                                 | Written as a condition around `:hover`.     |
| `@layer base` preflight                 | `preflight.css`                               | The same reset, in the `base` layer.        |
