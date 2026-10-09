import { glsl } from "../glsl/glsl.ts";
import { NOISE_TEXTURE } from "../shader/noise.ts";
import { PATTERN_SIZING, type Sizing, sizingUniforms } from "../shader/sizing.ts";
import { type ShaderColor, type ShaderValues } from "../shader/value.ts";
import type { ShaderProperties } from "../shader/shader.tsx";

/** The most colors in the warp. */
export const WARP_MAX_COLOR_COUNT = 10;

/** A pattern of the warp. */
export type WarpPattern = "checks" | "stripes" | "edge";

/** The number the fragment shader reads for each pattern. */
export const WARP_PATTERNS: Readonly<Record<WarpPattern, number>> = {
    checks: 0,
    stripes: 1,
    edge: 2,
};

/** The fragment shader of the warp. */
export const WARP_FRAGMENT = `#version 300 es
precision mediump float;

uniform float u_time;

uniform sampler2D u_noiseTexture;

uniform vec4 u_colors[${WARP_MAX_COLOR_COUNT}];
uniform float u_colorsCount;
uniform float u_proportion;
uniform float u_softness;
uniform float u_shape;
uniform float u_shapeScale;
uniform float u_distortion;
uniform float u_swirl;
uniform float u_swirlIterations;

in vec2 v_patternUV;

out vec4 fragColor;

${glsl.pi}
${glsl.rotate}
float randomG(vec2 p) {
  vec2 uv = floor(p) / 100. + .5;
  return texture(u_noiseTexture, fract(uv)).g;
}
float valueNoise(vec2 st) {
  vec2 i = floor(st);
  vec2 f = fract(st);
  float a = randomG(i);
  float b = randomG(i + vec2(1.0, 0.0));
  float c = randomG(i + vec2(0.0, 1.0));
  float d = randomG(i + vec2(1.0, 1.0));
  vec2 u = f * f * (3.0 - 2.0 * f);
  float x1 = mix(a, b, u.x);
  float x2 = mix(c, d, u.x);
  return mix(x1, x2, u.y);
}

void main() {
  vec2 uv = v_patternUV;
  uv *= .5;

  const float firstFrameOffset = 118.;
  float t = 0.0625 * (u_time + firstFrameOffset);

  float n1 = valueNoise(uv * 1. + t);
  float n2 = valueNoise(uv * 2. - t);
  float angle = n1 * TWO_PI;
  uv.x += 4. * u_distortion * n2 * cos(angle);
  uv.y += 4. * u_distortion * n2 * sin(angle);

  float swirl = u_swirl;
  for (int i = 1; i <= 20; i++) {
    if (i >= int(u_swirlIterations)) break;
    float iFloat = float(i);
    uv.x += swirl / iFloat * cos(t + iFloat * 1.5 * uv.y);
    uv.y += swirl / iFloat * cos(t + iFloat * 1. * uv.x);
  }

  float proportion = clamp(u_proportion, 0., 1.);

  float shape = 0.;
  if (u_shape < .5) {
    vec2 checksShape_uv = uv * (.5 + 3.5 * u_shapeScale);
    shape = .5 + .5 * sin(checksShape_uv.x) * cos(checksShape_uv.y);
    shape += .48 * sign(proportion - .5) * pow(abs(proportion - .5), .5);
  } else if (u_shape < 1.5) {
    vec2 stripesShape_uv = uv * (2. * u_shapeScale);
    float f = fract(stripesShape_uv.y);
    shape = smoothstep(.0, .55, f) * (1.0 - smoothstep(.45, 1., f));
    shape += .48 * sign(proportion - .5) * pow(abs(proportion - .5), .5);
  } else {
    float shapeScaling = 5. * (1. - u_shapeScale);
    float e0 = 0.45 - shapeScaling;
    float e1 = 0.55 + shapeScaling;
    shape = smoothstep(min(e0, e1), max(e0, e1), 1.0 - uv.y + 0.3 * (proportion - 0.5));
  }

  float mixer = shape * (u_colorsCount - 1.);
  vec4 gradient = u_colors[0];
  gradient.rgb *= gradient.a;
  float aa = fwidth(shape);
  for (int i = 1; i < ${WARP_MAX_COLOR_COUNT}; i++) {
    if (i >= int(u_colorsCount)) break;
    float m = clamp(mixer - float(i - 1), 0.0, 1.0);

    float localMixerStart = floor(m);
    float softness = .5 * u_softness + fwidth(m);
    float smoothed = smoothstep(max(0., .5 - softness - aa), min(1., .5 + softness + aa), m - localMixerStart);
    float stepped = localMixerStart + smoothed;

    m = mix(stepped, m, u_softness);

    vec4 c = u_colors[i];
    c.rgb *= c.a;
    gradient = mix(gradient, c, m);
  }

  vec3 color = gradient.rgb;
  float opacity = gradient.a;

  ${glsl.dither}

  fragColor = vec4(color, opacity);
}
`;

/** The options of the warp, beside its sizing. */
export interface WarpOptions extends Sizing, Pick<ShaderProperties, "speed" | "frame"> {
    /** Up to 10 gradient colors. */
    readonly colors?: readonly ShaderColor[];
    /** The blend point between colors, 0.5 = equal distribution, from 0 to 1. */
    readonly proportion?: number;
    /** The color transition sharpness, 0 = hard edge, 1 = smooth gradient, from 0 to 1. */
    readonly softness?: number;
    /** The strength of noise-based distortion, from 0 to 1. */
    readonly distortion?: number;
    /** The strength of the swirl distortion, needs swirlIterations > 1, from 0 to 1. */
    readonly swirl?: number;
    /** The number of layered swirl passes as an integer, needs swirl > 0, from 2 to 20. */
    readonly swirlIterations?: number;
    /** The zoom level of the base pattern, from 0 to 1. */
    readonly shapeScale?: number;
    /** The base pattern type. */
    readonly shape?: WarpPattern;
}

/** The properties of the warp, the shader's included. */
export interface WarpProperties
    extends WarpOptions, Omit<ShaderProperties, "fragmentShader" | "uniforms"> {}

/** The default warp. */
export const WARP_DEFAULTS: Required<WarpOptions> = {
    ...PATTERN_SIZING,
    rotation: 0,
    speed: 1,
    frame: 0,
    colors: ["#121212", "#9470ff", "#121212", "#8838ff"],
    proportion: 0.45,
    softness: 1,
    distortion: 0.25,
    swirl: 0.8,
    swirlIterations: 10,
    shapeScale: 0.1,
    shape: "checks",
};

/** Write warp options as the fragment shader's uniforms. */
export function warpUniforms(options: Required<WarpOptions>): ShaderValues {
    return {
        u_colors: options.colors,
        u_colorsCount: options.colors.length,
        u_proportion: options.proportion,
        u_softness: options.softness,
        u_distortion: options.distortion,
        u_swirl: options.swirl,
        u_swirlIterations: options.swirlIterations,
        u_shapeScale: options.shapeScale,
        u_shape: WARP_PATTERNS[options.shape],
        u_noiseTexture: NOISE_TEXTURE,
        ...sizingUniforms(options),
    };
}
