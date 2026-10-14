# @destack/style

`@destack/style` styles components with StyleX, and adds Tailwind's breakpoints in `/media.stylex` and Tailwind's preflight in `/preflight.css`.

```ts
style.attrs(styles.row); // StyleX attrs, typed for Solid
style.attributes(styles.row, { color: "red" }); // attrs, then an inline style
media.maxSm; // Tailwind max-sm:
media.pointerCoarse; // Tailwind pointer-coarse:
```

## Builds

`styleExtension` compiles the styles of a package and its dependencies into the `virtual:stylex.css` stylesheet.

```ts
import "virtual:stylex.css";
```

## Layers

`preflight.css` sits in a `base` layer below StyleX's priority layers, so every component style wins.

```css
@layer base, priority1, priority2;
```
