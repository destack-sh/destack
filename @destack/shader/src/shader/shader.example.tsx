import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { color } from "@destack/theme/tokens.stylex";
import { Shader } from "./shader.tsx";

/** A glow in the theme's primary color that breathes, as a custom fragment shader. */
const GLOW = `#version 300 es
precision mediump float;
uniform float u_time;
uniform vec4 u_color;
in vec2 v_objectUV;
out vec4 fragColor;
void main() {
  float glow = clamp(1. - length(v_objectUV) * (1.6 + .2 * sin(u_time)), 0., 1.);
  fragColor = u_color * glow;
}`;

/** The size every example draws at. */
const styles = style.create({
    canvas: { width: "100%", height: "20rem" },
});

/** A custom fragment shader drawing a glow in the theme's primary color. */
export const shaderGlow = defineExample({
    of: Shader,
    name: "glow",
    description: "a custom fragment shader drawing a glow in the theme's primary color",
    properties: { speed: 1 },
    render: (properties) => (
        <Shader
            fragment={GLOW}
            uniforms={{ u_color: color.primary }}
            {...properties}
            xstyle={styles.canvas}
        />
    ),
});
