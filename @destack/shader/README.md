# @destack/shader

Draw GPU shader effects in views, composed as layers, colored by the theme and still under reduced motion.

## Shader

`Shader` draws its effects as layers from bottom to top on a WebGPU canvas, and shows `fallback` where the device draws none.

```tsx
import { Aurora, Shader } from "@destack/shader";
import { color } from "@destack/theme/tokens.stylex";

<Shader xstyle={styles.hero} fallback={<img alt="" src="/hero.png" />}>
    <Aurora colorA={color.primary} colorB={color.accent} />
</Shader>;
// data-state="drawing", or "fallback" with onFailure("unsupported") where WebGPU is missing
```

## Effects

Every effect of the effect registry is a component typed by its own properties, each optional over its default, and an effect wraps the effects inside it.

```tsx
<Shader>
    <FilmGrain strength={0.2}>
        <WaveDistortion strength={0.3}>
            <RadialGradient colorA={color.primary} colorB={color.background} />
        </WaveDistortion>
    </FilmGrain>
</Shader>
```

## Custom effects

`defineEffect` makes a component of a custom effect from `defineShader`, written in the `shaders/std` words or raw WGSL, and the shader hands each custom definition it draws to the renderer.

```tsx
import { defineEffect, defineShader, Shader, transformColor, wgsl } from "@destack/shader";

const Halo = defineEffect(
    defineShader({
        name: "Halo",
        props: {
            inner: { default: "#ffd166", transform: transformColor },
            radius: { default: 0.6 },
        },
        paint: wgsl`return vec4f(inner.rgb * (1.0 - smoothstep(0.0, radius, length(uv - 0.5))), 1.0);`,
    }),
);

<Shader>
    <Halo inner={color.primary} />
</Shader>;
```

## Rendering

`colorSpace`, `toneMapping` and `observeElement` pass to the renderer, which builds the shader again when one changes, and `onReady` reports the first frame.

```tsx
<Shader colorSpace="p3-linear" toneMapping="aces" observeElement={false} onReady={() => reveal()}>
    <Aurora colorA={color.primary} />
</Shader>
// colorSpace "p3-linear" | "srgb"; toneMapping "linear" | "reinhard" | "aces" | "agx" | …
```

## Theme

A property naming a theme token, such as `color.primary`, draws the color the canvas computes, and the shader updates its layers when the device's scheme or contrast or a theme root above it changes.

```tsx
<RadialGradient colorA={color.primary} /> // var(--destack-color-primary) → "#3e63dd" in light, its dark color in dark
```

## Motion

A person who asked for reduced motion sees the first frame, paused.

```ts
matchMedia("(prefers-reduced-motion: reduce)").matches; // true
```
