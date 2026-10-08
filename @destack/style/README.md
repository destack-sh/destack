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

## Builds

`styleExtension` compiles StyleX styles into the output's stylesheet in builds of every package whose dependency closure includes `@destack/style`.

```ts
import { Button } from "@destack/ui/button"; // the build compiles the button's styles through @destack/ui's dependency on @destack/style
```

## Layers

StyleX emits styles into the `priority1` to `priority10` cascade layers, and `styleExtension` orders a `base` layer before them for element defaults, so any component style wins.

```css
@layer base, priority1, priority2; /* … priority10 */
```

## Preflight

`preflight.css` resets the browser's element styles in the `base` layer: boxes size by their border, margins and borders clear, lists unstyle, media fits its container, and controls take the text they sit in.

```tsx
import "@destack/style/preflight.css";

<button {...style.attrs(styles.button)}>Save</button>; // no browser border, background or font to undo
```
