# @destack/shader

Draw fragment shader effects in views on WebGL 2, colored by the theme and held still under reduced motion.

## Effects

Each effect is a component of its own export, which draws with its defaults and takes theme tokens as colors.

```tsx
import { Metaballs } from "@destack/shader/metaballs";
import { color } from "@destack/theme/tokens.stylex";

<Metaballs colors={[color.primary, color.accent]} colorBack={color.background} count={12} speed={0.5} />;
```

## Sizing

`fit`, `scale`, `rotation`, `offsetX` and `offsetY` place an effect's graphic in its canvas.

```tsx
<MeshGradient fit="cover" scale={1.4} rotation={30} offsetX={-0.2} />;
```

## Images

An image effect samples `image`, an address or an element, and draws without one where its fragment shader allows.

```tsx
<FlutedGlass image="/cover.jpg" colorHighlight={color.primary} />;
```

## Custom shaders

`Shader` draws any GLSL ES 3.0 fragment shader with its uniforms: numbers, vectors, CSS colors and theme tokens as `vec4`, and images as samplers.

```tsx
import { Shader } from "@destack/shader";

const glow = `#version 300 es
precision mediump float;
uniform float u_time;
uniform vec4 u_color;
in vec2 v_objectUV;
out vec4 fragColor;
void main() { fragColor = u_color * (1. - length(v_objectUV) * (1.5 + .2 * sin(u_time))); }`;

<Shader fragment={glow} uniforms={{ u_color: color.primary }} speed={1} />;
```

## Fallback

`fallback` shows where the browser draws no WebGL 2, and `onFailure` reports why.

```tsx
<Shader fragment={glow} uniforms={{}} fallback={<img alt="" src="/glow.png" />} onFailure={report} />;
// "unsupported" | "compile" | "link"
```

## Resolution

A shader renders at the canvas's device pixels, and the page's shaders lower their resolution together while frames run slow.

```tsx
<Shader fragment={glow} uniforms={{}} minPixelRatio={1} maxPixelCount={1920 * 1080} resolution="fixed" />;
```

## Motion

`speed` scales animation time, `frame` sets it, and a person who asked for reduced motion sees one still frame.

```tsx
<Waves speed={0} frame={2400} />;
```
