# @destack/shader

Draw fragment shader effects in views, colored by the theme.

## Overview

`@destack/shader` is Paper Shaders for Solid: ready-made effects, custom fragment shaders, and theme colors that follow the person's theme.

Views draw effects:

- Effects, such as `MeshGradient`, `Metaballs` and `Waves`: ready-made shaders, one per entry.
- `Shader`: a custom GLSL ES 3.0 fragment shader with its uniforms.
- `ShaderMount`: the running shader, drawn in WebGL2.

Draw an effect in the theme's colors:

```tsx
import { Metaballs } from "@destack/shader/metaballs";
import { color } from "@destack/theme/tokens.stylex";

// draw metaballs in the theme's colors
export const Background = () => (
    <Metaballs colors={[color.primary, color.accent]} colorBack={color.background} speed={0.5} />
);
```

## Reference

### Effects

Each effect is a component at `@destack/shader/<effect>`, with its `colors`, `speed`, `frame`, `fit` and `scale`.
The effects are `dithering`, `dot-grid`, `fluted-glass`, `god-rays`, `grain-gradient`, `halftone-cmyk`, `halftone-dots`, `image-dithering`, `mesh-gradient`, `metaballs`, `paper-texture`, `perlin-noise`, `pulsing-border`, `simplex-noise`, `static-mesh-gradient`, `static-radial-gradient`, `voronoi`, `warp`, `water`, `waves`.

### Shader

`Shader` draws a fragment shader, held still where the person reduces motion.

| Property                         | What it is                                                     |
| -------------------------------- | -------------------------------------------------------------- |
| `fragmentShader`                 | the GLSL ES 3.0 source                                         |
| `uniforms`                       | numbers, vectors, CSS colors, theme tokens or images, by name  |
| `speed`, `frame`                 | the speed of time, 0 to hold a frame, and the time drawn in ms |
| `minPixelRatio`, `maxPixelCount` | the render resolution's bounds                                 |
| `resolution`                     | whether slow frames lower the resolution                       |
| `mipmaps`                        | the image uniforms that get mipmaps                            |
| `fallback`, `onFailure`          | what shows, and who hears, where the shader can't draw         |

A shader reads its time as `u_time`, and its coordinates as `v_objectUV`, `v_responsiveUV`, `v_patternUV` and `v_imageUV`, with their box sizes.

## Examples

### Draw a custom shader

A theme token colors a uniform.

```tsx
import { Shader } from "@destack/shader";
import { color } from "@destack/theme/tokens.stylex";

// glow from the center, pulsing over time
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

// draw it in the primary color
export const Halo = () => <Shader fragmentShader={glow} uniforms={{ u_color: color.primary }} />;
```

## Comparison

### Paper Shaders

Effects, their options and the shader mount follow Paper Shaders.

| Paper Shaders    | `@destack/shader` | Difference                                     |
| ---------------- | ----------------- | ---------------------------------------------- |
| React components | Solid components  | The same effects and options.                  |
| CSS colors       | theme tokens      | Colors follow the person's theme.              |
| `speed`          | `speed`           | Every effect holds still under reduced motion. |
| none             | `fallback`        | Shows where WebGL2 can't draw.                 |
