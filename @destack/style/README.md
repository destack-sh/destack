# @destack/style

Define styles with StyleX.

## Styles

`create` declares named styles, and the build compiles each property and value to one CSS class.

```ts
import * as style from "@destack/style";

const styles = style.create({ container: { display: "flex" } });
```

## Attributes

`attrs` reads the class names and dynamic values an element takes, and `attributes` applies a caller's inline style after them.

```tsx
const styles = style.create({ row: (row: number) => ({ gridRow: row }) });

<div {...style.attrs(styles.row(3))} />; // class="x33j1x8" style="--x-gridRow:3"
<div {...style.attributes(styles.row(3), { "pointer-events": "none" })} />;
```

## Variables and themes

`defineVars` declares custom properties, `createTheme` overrides them under a class, and `defineConsts` inlines values at build time.

```ts
const colors = style.defineVars({ accent: "blue" });
const night = style.createTheme(colors, { accent: "teal" });

<div {...style.attrs(night)} />; // descendants read --accent as teal
```

## Conditions

`media` holds the shared media conditions, the breakpoints and variants of Tailwind, and `when` styles an element by an ancestor's, sibling's or descendant's state.

```ts
import { media } from "@destack/style/media.stylex";

const styles = style.create({
    panel: { display: { default: "none", [media.md]: "grid" } }, // grid from 48rem
    link: { color: { default: "gray", [style.when.ancestor(":hover")]: "black" } },
});
// sm md lg xl 2xl, maxSm … max2xl, hover pointerCoarse motionReduce motionSafe contrastMore print portrait landscape
```

## Animations

`keyframes` declares an animation, `positionTry` a fallback position for anchored elements, and `viewTransitionClass` the styles of a view transition.

```ts
const fade = style.keyframes({ from: { opacity: 0 }, to: { opacity: 1 } });
const styles = style.create({ shown: { animationName: fade, animationDuration: "150ms" } });
```

## Builds

`styleExtension` compiles StyleX styles into the output's stylesheet in builds of every package whose dependency closure includes `@destack/style`, and serves them in development as the `virtual:stylex.css` module, reloaded as rules grow.

```ts
import "virtual:stylex.css"; // the collected rules, a module in development and the entry stylesheet in builds
import { Button } from "@destack/ui/button"; // the build compiles the button's styles through @destack/ui's dependency on @destack/style
```

## Layers

StyleX emits styles into the `priority1` to `priority10` cascade layers, and `styleExtension` orders a `base` layer before them for element defaults, so any component style wins.

```css
@layer base, priority1, priority2; /* … priority10 */
```

## Preflight

`preflight.css` resets the browser's element styles in the `base` layer, after Tailwind's preflight: boxes size by their border, margins and borders clear, lists unstyle, media fits its container, and controls take the text they sit in.

```tsx
import "@destack/style/preflight.css";

<button {...style.attrs(styles.button)}>Save</button>; // no browser border, background or font to undo
```
