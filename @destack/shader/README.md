# @destack/shader

Draw fragment shader effects in views, colored by the theme.

## Overview

Each effect is a Solid component drawing a WebGL2 fragment shader, colored with theme tokens and held still where the person reduces motion.

- **Effect**: a ready-made shader such as `MeshGradient`, `Metaballs` or `Waves`, at `@destack/shader/<effect>`, with its `colors`, `speed`, `frame`, `fit` and `scale`.
- **Shader**: a custom GLSL ES 3.0 fragment shader with its uniforms: numbers, vectors, CSS colors, theme tokens or images; a `fallback` shows where it cannot draw.
- **ShaderMount**: the running shader; its `frame` is the time the shader reads as `u_time`.

```tsx
import { Metaballs } from "@destack/shader/metaballs";
import { color } from "@destack/theme/tokens.stylex";

const Background = () => (
    <Metaballs colors={[color.primary, color.accent]} colorBack={color.background} speed={0.5} />
);
```

## Examples

### Draw a custom shader

`Shader` draws a fragment shader with its uniforms, which a theme token colors.

```tsx
import { Shader } from "@destack/shader";
import { color } from "@destack/theme/tokens.stylex";

const glow = `#version 300 es
precision mediump float;
uniform float u_time;
uniform vec4 u_color;
in vec2 v_objectUV;
out vec4 fragColor;
void main() {
  float glow = clamp(1. - length(v_objectUV) * (1.6 + .2 * sin(u_time)), 0., 1.);
  fragColor = u_color * glow;
}`;

const Halo = () => <Shader fragmentShader={glow} uniforms={{ u_color: color.primary }} />;
```

### Show one still frame

`speed: 0` stops the effect at its `frame`, as reduced motion does.

```tsx
import { Waves } from "@destack/shader/waves";

const Still = () => <Waves speed={0} frame={2400} />;
```

## Comparison

### Paper Shaders

Effects, their options and the shader mount follow Paper Shaders; three things differ.

- Effects are Solid components, colored with theme tokens that follow the person's theme.
- Every effect shows one still frame where the person reduces motion.
- A shader that cannot draw shows its `fallback` and reports through `onFailure`.
