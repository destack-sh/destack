import { glsl } from "../glsl/glsl.ts";
import { PATTERN_SIZING, type Sizing, sizingUniforms } from "../shader/sizing.ts";
import { type ShaderColor, type ShaderValues } from "../shader/value.ts";
import type { ShaderProperties } from "../shader/shader.tsx";

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

/** The options of the waves, beside its sizing. */
export interface WavesOptions extends Sizing, Pick<ShaderProperties, "speed" | "frame"> {
    /** The foreground color. */
    readonly colorFront?: ShaderColor;
    /** The background color. */
    readonly colorBack?: ShaderColor;
    /** The line shape, 0 = zigzag, 1 = sine, 2-3 = irregular waves, fractional values morph between shapes, needs amplitude > 0, frequency > 0, from 0 to 3. */
    readonly shape?: number;
    /** The wave frequency, needs amplitude > 0, from 0 to 2. */
    readonly frequency?: number;
    /** The wave amplitude, at frequency = 0 it only shifts the lines, from 0 to 1. */
    readonly amplitude?: number;
    /** The space between every two wavy lines, from 0 to 2. */
    readonly spacing?: number;
    /** The blend point between front and back colors, 0.5 = equal distribution, from 0 to 1. */
    readonly proportion?: number;
    /** The color transition sharpness, 0 = hard edge, 1 = smooth gradient, from 0 to 1. */
    readonly softness?: number;
}

/** The properties of the waves, the shader's included. */
export interface WavesProperties
    extends WavesOptions, Omit<ShaderProperties, "fragmentShader" | "uniforms"> {}

/** The default waves. */
export const WAVES_DEFAULTS: Required<WavesOptions> = {
    speed: 0,
    frame: 0,
    ...PATTERN_SIZING,
    scale: 0.6,
    colorFront: "#ffbb00",
    colorBack: "#000000",
    shape: 0,
    frequency: 0.5,
    amplitude: 0.5,
    spacing: 1.2,
    proportion: 0.1,
    softness: 0,
};

/** Write waves options as the fragment shader's uniforms. */
export function wavesUniforms(options: Required<WavesOptions>): ShaderValues {
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
