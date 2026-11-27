# @destack/style

Describe colors and styles as the web does, and style components with StyleX.

## Overview

`@destack/style` is CSS's value types as schemas, and StyleX with Tailwind's media queries and preflight, all from one module.
Values follow CSS's value types and CSS Color 4, so CSS reads them as they are.
Each choice among a few values, such as a blend mode, is a schema enum, with its titles beside it.

Values:

- `Color`: a CSS color in sRGB, HSL, HSB or OKLCH, read from and written as CSS text.
- `Hue`: ten hues in Radix Colors' values, which a theme may replace.
- `Gradient`: color stops in a shape, mixed in a CSS color space, as `linear-gradient(in oklab, …)` mixes them.
- `Palette`: a list of colors.
- `Paint`: what fills one layer of a box: a color, a gradient or an image.
- `FillStack`: a box's layers of paint, top first, as CSS lists background layers.

Styles:

- `create`: declares a component's styles, as in StyleX.
- `attrs` and `attributes`: apply styles as `class` and `style` attributes.
- `defineVars` and `createTheme`: declare CSS custom properties, and override them under an element.
- `media` and `frame`: Tailwind's breakpoints and a page's layout grid, as StyleX constants and variables.
- `preflight.css`: Tailwind's preflight, below every component style.
- `styleExtension`: compiles each package's styles in builds.

Make a hue lighter as people see it, and style a row with it:

```ts
import * as style from "@destack/style";

// make cyan lighter as people see it
const cyan = style.Color.parse(style.DEFAULT_HUES.cyan);
const light = cyan.to("oklch").with("lightness", 80).toString();

// declare a row in that color, and apply it
const styles = style.create({ row: { display: "flex", backgroundColor: light } });
const attributes = style.attrs(styles.row);
```

## Reference

### Colors

A `Color` is one color in a space, read from CSS text and written back as hex, `rgb()`, `hsl()` or `oklch()`.
`Color.schema` checks a CSS color string, and has the id `color`, so inputs show a color picker.

| Verb                         | What it does                                        |
| ---------------------------- | --------------------------------------------------- |
| `Color.parse(text)`          | reads a CSS color, failing on any other text        |
| `Color.read(text)`           | reads a CSS color, undefined for any other text     |
| `color.to(space)`            | converts it to `srgb`, `hsl`, `hsb` or `oklch`      |
| `color.with(channel, value)` | changes one channel, such as `lightness` or `alpha` |
| `color.name()`               | names its hue and lightness, as a person says them  |
| `color.toString(format)`     | writes it as `hex`, `rgb`, `hsl` or `oklch`         |

### Hues

A `Hue` is one of ten hues that mean things in a view, such as `red` for danger.
`DEFAULT_HUES` has Radix Colors' value for each, and `Hue.choices` titles each in the person's language.

| Value          | What it holds                                         |
| -------------- | ----------------------------------------------------- |
| `HUES`         | the ten hues, in the order pickers show them          |
| `DEFAULT_HUES` | each hue's color without a theme                      |
| `Hue.choices`  | each hue as a choice, titled in the person's language |

### Gradients

A `Gradient` is two or more stops, each a color at a position from 0 to 1, spread in a shape and mixed in a space.
It mixes as CSS Color 4 does, with premultiplied alpha, and has the id `gradient`.

| Verb                             | What it does                                       |
| -------------------------------- | -------------------------------------------------- |
| `Gradient.between(from, to)`     | makes a gradient to the right in OKLab             |
| `gradient.at(position)`          | mixes the color at a position, as `color-mix` does |
| `gradient.first()`               | reads the stop at its start                        |
| `gradient.add(position)`         | adds a stop with the color already there           |
| `gradient.move(index, position)` | moves a stop to another position                   |
| `gradient.recolor(index, color)` | gives a stop another color                         |
| `gradient.remove(index)`         | removes a stop, keeping at least two               |
| `gradient.blend(space, hue)`     | mixes it in another space and hue direction        |
| `gradient.reshape(shape)`        | spreads its stops in another shape                 |
| `gradient.to(kind)`              | changes the shape's kind, keeping angle and center |
| `gradient.css()`                 | writes it as the CSS function its shape draws      |

| Shape                              | CSS                                                |
| ---------------------------------- | -------------------------------------------------- |
| `{ kind: "linear", angle }`        | `linear-gradient(90deg in oklab, …)`               |
| `{ kind: "radial", center }`       | `radial-gradient(at 50% 50% in oklab, …)`          |
| `{ kind: "conic", center, angle }` | `conic-gradient(from 0deg at 50% 50% in oklab, …)` |

| Space   | Hue       | CSS                                       |
| ------- | --------- | ----------------------------------------- |
| `oklab` |           | `linear-gradient(in oklab, …)`            |
| `oklch` | `shorter` | `linear-gradient(in oklch, …)`            |
| `oklch` | `longer`  | `linear-gradient(in oklch longer hue, …)` |
| `srgb`  |           | `linear-gradient(in srgb, …)`             |

### Paints and fills

A `Paint` fills one layer of a box: `solid` with a color, `gradient`, or `image` with an address, a fit and a position.
A `FillStack` lists a box's layers top first, each a paint with a blend mode, and has the id `fillStack`.

| Verb                    | What it does                                                       |
| ----------------------- | ------------------------------------------------------------------ |
| `Paint.black`           | the paint a box and a new layer start at                           |
| `Paint.layer(paint)`    | draws a paint as one CSS background layer                          |
| `Paint.color(paint)`    | reads the color a paint turns into                                 |
| `Paint.gradient(paint)` | reads the gradient a paint turns into, a color fading out          |
| `Paint.image(source)`   | makes an image paint covering the box from its center              |
| `FillStack.css(stack)`  | draws the shown layers as `background-image`, `-size` and the rest |

```ts
import { FillStack } from "@destack/style";

// tint a sheet of paper blue
const fill = FillStack.css([
    { paint: { kind: "solid", color: "#0000ff33" }, blend: "multiply", isVisible: true },
    { paint: { kind: "solid", color: "#fffdf7" }, blend: "normal", isVisible: true },
]);
```

### Palettes

A `Palette` is a list of colors with the id `palette`, narrowed as any array schema is.

```ts
import { Palette } from "@destack/style";

// take three to eight colors
const Swatches = Palette.min(3).max(8);
```

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
