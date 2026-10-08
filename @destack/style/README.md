# @destack/style

`@destack/style` is StyleX (`create`, `props`, `defineVars`, `createTheme`, `keyframes`, `when`…), `@destack/style/media.stylex` is Tailwind's breakpoints and variants, and `@destack/style/preflight.css` is Tailwind's preflight.

```ts
style.attrs(styles.row); // StyleX attrs, typed for Solid
style.attributes(styles.row, { color: "red" }); // attrs, then an inline style
media.maxSm; // Tailwind max-sm:
media.pointerCoarse; // Tailwind pointer-coarse:
```

## Builds

`styleExtension` compiles the styles of a package and every dependency into the stylesheet its entry loads, served as `virtual:stylex.css` in development.

```ts
import "virtual:stylex.css";
```

## Layers

`preflight.css` sits in a `base` layer below StyleX's priority layers, so every component style wins.

```css
@layer base, priority1, priority2;
```
