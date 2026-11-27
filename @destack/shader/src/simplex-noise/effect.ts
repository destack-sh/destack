import { Palette } from "@destack/style";
import { t } from "@destack/locale";
import { field } from "@destack/object";
import { schema } from "@destack/schema";
import type { DeclaredProperties } from "@destack/view/declare";
import { Shader } from "../shader/shader.tsx";
import { PATTERN_SIZING_PROPERTIES, sizingUniforms } from "../shader/sizing.ts";
import { SOFTNESS } from "../shader/option.ts";
import { glsl } from "../glsl/glsl.ts";
import { type ShaderValues } from "../shader/value.ts";

/** The most colors in the simplex noise. */
export const SIMPLEX_NOISE_MAX_COLOR_COUNT = 10;

/** The fragment shader of the simplex noise. */
export const SIMPLEX_NOISE_FRAGMENT = `#version 300 es
precision mediump float;

uniform float u_time;

uniform vec4 u_colors[${SIMPLEX_NOISE_MAX_COLOR_COUNT}];
uniform float u_colorsCount;
uniform float u_stepsPerColor;
uniform float u_softness;

in vec2 v_patternUV;

out vec4 fragColor;

${glsl.simplex}

float getNoise(vec2 uv, float t) {
  float noise = .5 * snoise(uv - vec2(0., .3 * t));
  noise += .5 * snoise(2. * uv + vec2(0., .32 * t));

  return noise;
}

float steppedSmooth(float m, float steps, float softness) {
  float stepT = floor(m * steps) / steps;
  float f = m * steps - floor(m * steps);
  float fw = steps * fwidth(m);
  float smoothed = smoothstep(.5 - softness, min(1., .5 + softness + fw), f);
  return stepT + smoothed / steps;
}

void main() {
  vec2 shape_uv = v_patternUV;
  shape_uv *= .1;

  float t = .2 * u_time;

  float shape = .5 + .5 * getNoise(shape_uv, t);

  float mixer = (shape - .5 / u_colorsCount) * u_colorsCount;

  float steps = max(1., u_stepsPerColor);

  vec4 gradient = u_colors[0];
  gradient.rgb *= gradient.a;
  for (int i = 1; i < ${SIMPLEX_NOISE_MAX_COLOR_COUNT}; i++) {
    if (i >= int(u_colorsCount)) break;

    float localM = clamp(mixer - float(i - 1), 0., 1.);
    localM = steppedSmooth(localM, steps, .5 * u_softness);

    vec4 c = u_colors[i];
    c.rgb *= c.a;
    gradient = mix(gradient, c, localM);
  }

  if ((mixer < 0.) || (mixer > (u_colorsCount - 1.))) {
    float localM = mixer + 1.;
    if (mixer > (u_colorsCount - 1.)) {
      localM = mixer - (u_colorsCount - 1.);
    }
    localM = steppedSmooth(localM, steps, .5 * u_softness);
    vec4 cFst = u_colors[0];
    cFst.rgb *= cFst.a;
    vec4 cLast = u_colors[int(u_colorsCount - 1.)];
    cLast.rgb *= cLast.a;
    gradient = mix(cLast, cFst, localM);
  }

  vec3 color = gradient.rgb;
  float opacity = gradient.a;

  ${glsl.dither}

  fragColor = vec4(color, opacity);
}
`;

/** The properties people change in the simplex noise's controls and inspector. */
export const SIMPLEX_NOISE_PROPERTIES = {
    ...Shader.properties,
    speed: Shader.properties.speed.default(0.5),
    ...PATTERN_SIZING_PROPERTIES,
    scale: PATTERN_SIZING_PROPERTIES.scale.default(0.6),
    colors: field
        .json(Palette.min(1).max(SIMPLEX_NOISE_MAX_COLOR_COUNT))
        .labeled(t.context("label")`Colors`)
        .described(t`The colors the effect blends, in order.`)
        .default(["#4449CF", "#FFD1E0", "#F94446", "#FFD36B", "#FFFFFF"]),
    stepsPerColor: field
        .integer(schema.number().int().min(1).max(10))
        .labeled(t.context("label")`Steps per color`)
        .described(t`How many colors lie between each two given ones.`)
        .default(2),
    softness: SOFTNESS.default(0),
};

/** The options of the simplex noise, each with its value. */
export type SimplexNoiseOptions = DeclaredProperties<typeof SIMPLEX_NOISE_PROPERTIES>;

/** Write simplex noise options as the fragment shader's uniforms. */
export function simplexNoiseUniforms(options: SimplexNoiseOptions): ShaderValues {
    return {
        u_colors: options.colors,
        u_colorsCount: options.colors.length,
        u_stepsPerColor: options.stepsPerColor,
        u_softness: options.softness,
        ...sizingUniforms(options),
    };
}
