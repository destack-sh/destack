import { t } from "@destack/locale";
import { field } from "@destack/object";
import { schema } from "@destack/schema";
import type { DeclaredProperties } from "@destack/view/declare";
import { Shader } from "../shader/shader.tsx";
import { PATTERN_SIZING_PROPERTIES, sizingUniforms } from "../shader/sizing.ts";
import { COLOR_BACK, COLOR_FRONT, PROPORTION, SOFTNESS } from "../shader/option.ts";
import { glsl } from "../glsl/glsl.ts";
import { type ShaderValues } from "../shader/value.ts";

/** The fragment shader of the waves. */
export const WAVES_FRAGMENT = `#version 300 es
precision mediump float;

uniform vec4 u_colorFront;
uniform vec4 u_colorBack;
uniform float u_shape;
uniform float u_frequency;
uniform float u_amplitude;
uniform float u_spacing;
uniform float u_proportion;
uniform float u_softness;

in vec2 v_patternUV;

out vec4 fragColor;

${glsl.pi}

void main() {
  vec2 shape_uv = v_patternUV;
  shape_uv *= 4.;

  float wave = .5 * cos(shape_uv.x * u_frequency * TWO_PI);
  float zigzag = 2. * abs(fract(shape_uv.x * u_frequency) - .5);
  float irregular = sin(shape_uv.x * .25 * u_frequency * TWO_PI) * cos(shape_uv.x * u_frequency * TWO_PI);
  float irregular2 = .75 * (sin(shape_uv.x * u_frequency * TWO_PI) + .5 * cos(shape_uv.x * .5 * u_frequency * TWO_PI));

  float offset = mix(zigzag, wave, smoothstep(0., 1., u_shape));
  offset = mix(offset, irregular, smoothstep(1., 2., u_shape));
  offset = mix(offset, irregular2, smoothstep(2., 3., u_shape));
  offset *= 2. * u_amplitude;

  float spacing = (.001 + u_spacing);
  float shape = .5 + .5 * sin((shape_uv.y + offset) * PI / spacing);

  float aa = .0001 + fwidth(shape);
  float dc = 1. - clamp(u_proportion, 0., 1.);
  float e0 = dc - u_softness - aa;
  float e1 = dc + u_softness + aa;
  float res = smoothstep(min(e0, e1), max(e0, e1), shape);

  vec3 fgColor = u_colorFront.rgb * u_colorFront.a;
  float fgOpacity = u_colorFront.a;
  vec3 bgColor = u_colorBack.rgb * u_colorBack.a;
  float bgOpacity = u_colorBack.a;

  vec3 color = fgColor * res;
  float opacity = fgOpacity * res;

  color += bgColor * (1. - opacity);
  opacity += bgOpacity * (1. - opacity);

  fragColor = vec4(color, opacity);
}
`;

/** The properties people change in the waves's controls and inspector. */
export const WAVES_PROPERTIES = {
    ...Shader.properties,
    ...PATTERN_SIZING_PROPERTIES,
    scale: PATTERN_SIZING_PROPERTIES.scale.default(0.6),
    colorFront: COLOR_FRONT.default("#ffbb00"),
    colorBack: COLOR_BACK.default("#000000"),
    shape: field
        .number(schema.number().min(0).max(3))
        .labeled(t.context("label")`Shape`)
        .described(t`The shape of the lines, from straight to sharp.`)
        .default(0),
    frequency: field
        .number(schema.number().min(0).max(2))
        .labeled(t.context("label")`Frequency`)
        .described(t`How often the waves repeat.`)
        .default(0.5),
    amplitude: field
        .number(schema.number().min(0).max(1))
        .labeled(t.context("label")`Amplitude`)
        .described(t`How tall the waves are.`)
        .default(0.5),
    spacing: field
        .number(schema.number().min(0).max(2))
        .labeled(t.context("label")`Spacing`)
        .described(t`The space between two lines.`)
        .default(1.2),
    proportion: PROPORTION.default(0.1),
    softness: SOFTNESS.default(0),
};

/** The options of the waves, each with its value. */
export type WavesOptions = DeclaredProperties<typeof WAVES_PROPERTIES>;

/** Write waves options as the fragment shader's uniforms. */
export function wavesUniforms(options: WavesOptions): ShaderValues {
    return {
        u_colorFront: options.colorFront,
        u_colorBack: options.colorBack,
        u_shape: options.shape,
        u_frequency: options.frequency,
        u_amplitude: options.amplitude,
        u_spacing: options.spacing,
        u_proportion: options.proportion,
        u_softness: options.softness,
        ...sizingUniforms(options),
    };
}
