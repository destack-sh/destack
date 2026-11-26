import { Palette } from "@destack/color";
import { t } from "@destack/locale";
import { field } from "@destack/object";
import { schema } from "@destack/schema";
import type { DeclaredProperties } from "@destack/view/declare";
import { Shader } from "../shader/shader.tsx";
import { PATTERN_SIZING_PROPERTIES, sizingUniforms } from "../shader/sizing.ts";
import { PROPORTION } from "../shader/option.ts";
import { glsl } from "../glsl/glsl.ts";
import { NOISE_TEXTURE } from "../shader/noise.ts";
import { type ShaderValues } from "../shader/value.ts";

/** The most colors in the warp. */
export const WARP_MAX_COLOR_COUNT = 10;

/** The patterns of the warp, as a person picks one. */
const WARP_PATTERN_CHOICES = {
    checks: { title: t.context("option")`Checks` },
    stripes: { title: t.context("option")`Stripes` },
    edge: { title: t.context("option")`Edge` },
};

/** A pattern of the warp. */
export type WarpPattern = keyof typeof WARP_PATTERN_CHOICES;

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

/** The properties people change in the warp's controls and inspector. */
export const WARP_PROPERTIES = {
    ...Shader.properties,
    speed: Shader.properties.speed.default(1),
    ...PATTERN_SIZING_PROPERTIES,
    colors: field
        .json(Palette.min(1).max(WARP_MAX_COLOR_COUNT))
        .labeled(t.context("label")`Colors`)
        .described(t`The colors the effect blends, in order.`)
        .default(["#121212", "#9470ff", "#121212", "#8838ff"]),
    proportion: PROPORTION.default(0.45),
    softness: field
        .number(schema.number().min(0).max(2))
        .labeled(t.context("label")`Softness`)
        .described(t`How smoothly the colors blend, from a hard edge to a smooth gradient.`)
        .default(1),
    distortion: field
        .number(schema.number().min(0).max(1))
        .labeled(t.context("label")`Distortion`)
        .described(t`How strongly noise bends the pattern.`)
        .default(0.25),
    swirl: field
        .number(schema.number().min(0).max(1))
        .labeled(t.context("label")`Swirl`)
        .described(t`How strongly the pattern swirls.`)
        .default(0.8),
    swirlIterations: field
        .integer(schema.number().int().min(2).max(20))
        .labeled(t.context("label")`Swirl passes`)
        .described(t`How many swirls lie over each other.`)
        .default(10),
    shapeScale: field
        .number(schema.number().min(0).max(1))
        .labeled(t.context("label")`Pattern scale`)
        .described(t`The size of the base pattern.`)
        .default(0.1),
    shape: field
        .enum(WARP_PATTERN_CHOICES)
        .labeled(t.context("label")`Shape`)
        .described(t`The base pattern.`)
        .default("checks"),
};

/** The options of the warp, each with its value. */
export type WarpOptions = DeclaredProperties<typeof WARP_PROPERTIES>;

/** Write warp options as the fragment shader's uniforms. */
export function warpUniforms(options: WarpOptions): ShaderValues {
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
