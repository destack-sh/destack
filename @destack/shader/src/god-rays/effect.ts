import { Color, Palette } from "@destack/color";
import { t } from "@destack/locale";
import { field } from "@destack/object";
import { schema } from "@destack/schema";
import type { DeclaredProperties } from "@destack/view/declare";
import { Shader } from "../shader/shader.tsx";
import { SIZING_PROPERTIES, sizingUniforms } from "../shader/sizing.ts";
import { COLOR_BACK } from "../shader/option.ts";
import { glsl } from "../glsl/glsl.ts";
import { NOISE_TEXTURE } from "../shader/noise.ts";
import { type ShaderValues } from "../shader/value.ts";

/** The most colors in the god rays. */
export const GOD_RAYS_MAX_COLOR_COUNT = 5;

/** The fragment shader of the god rays. */
export const GOD_RAYS_FRAGMENT = `#version 300 es
precision mediump float;

uniform float u_time;

uniform sampler2D u_noiseTexture;

uniform vec4 u_colorBack;
uniform vec4 u_colorBloom;
uniform vec4 u_colors[${GOD_RAYS_MAX_COLOR_COUNT}];
uniform float u_colorsCount;

uniform float u_density;
uniform float u_spotty;
uniform float u_midSize;
uniform float u_midIntensity;
uniform float u_intensity;
uniform float u_bloom;

in vec2 v_objectUV;

out vec4 fragColor;

${glsl.pi}
${glsl.rotate}
${glsl.randomR}
float valueNoise(vec2 st) {
  vec2 i = floor(st);
  vec2 f = fract(st);
  float a = randomR(i);
  float b = randomR(i + vec2(1.0, 0.0));
  float c = randomR(i + vec2(0.0, 1.0));
  float d = randomR(i + vec2(1.0, 1.0));
  vec2 u = f * f * (3.0 - 2.0 * f);
  float x1 = mix(a, b, u.x);
  float x2 = mix(c, d, u.x);
  return mix(x1, x2, u.y);
}

${glsl.hash11}

float raysShape(vec2 uv, float r, float freq, float intensity, float radius) {
  float a = atan(uv.y, uv.x);
  vec2 left = vec2(a * freq, r);
  vec2 right = vec2(fract(a / TWO_PI) * TWO_PI * freq, r);
  float n_left = pow(valueNoise(left), intensity);
  float n_right = pow(valueNoise(right), intensity);
  float shape = mix(n_right, n_left, smoothstep(-.15, .15, uv.x));
  return shape;
}

void main() {
  vec2 shape_uv = v_objectUV;

  float t = .2 * u_time;

  float radius = length(shape_uv);
  float spots = 6.5 * abs(u_spotty);

  float intensity = 4. - 3. * clamp(u_intensity, 0., 1.);

  float delta = 1. - smoothstep(0., 1., radius);

  float midSize = 10. * abs(u_midSize);
  float ms_lo = 0.02 * midSize;
  float ms_hi = max(midSize, 1e-6);
  float middleShape = pow(u_midIntensity, 0.3) * (1. - smoothstep(ms_lo, ms_hi, 3.0 * radius));
  middleShape = pow(middleShape, 5.0);

  vec3 accumColor = vec3(0.0);
  float accumAlpha = 0.0;

  for (int i = 0; i < ${GOD_RAYS_MAX_COLOR_COUNT}; i++) {
    if (i >= int(u_colorsCount)) break;

    vec2 rotatedUV = rotate(shape_uv, float(i) + 1.0);

    float r1 = radius * (1.0 + 0.4 * float(i)) - 3.0 * t;
    float r2 = 0.5 * radius * (1.0 + spots) - 2.0 * t;
    float density = 6. * u_density + step(.5, u_density) * pow(4.5 * (u_density - .5), 4.);
    float f = mix(1.0, 3.0 + 0.5 * float(i), hash11(float(i) * 15.)) * density;

    float ray = raysShape(rotatedUV, r1, 5.0 * f, intensity, radius);
    ray *= raysShape(rotatedUV, r2, 4.0 * f, intensity, radius);
    ray += (1. + 4. * ray) * middleShape;
    ray = clamp(ray, 0.0, 1.0);

    float srcAlpha = u_colors[i].a * ray;
    vec3 srcColor = u_colors[i].rgb * srcAlpha;

    vec3 alphaBlendColor = accumColor + (1.0 - accumAlpha) * srcColor;
    float alphaBlendAlpha = accumAlpha + (1.0 - accumAlpha) * srcAlpha;

    vec3 addBlendColor = accumColor + srcColor;
    float addBlendAlpha = accumAlpha + srcAlpha;

    accumColor = mix(alphaBlendColor, addBlendColor, u_bloom);
    accumAlpha = mix(alphaBlendAlpha, addBlendAlpha, u_bloom);
  }

  float overlayAlpha = u_colorBloom.a;
  vec3 overlayColor = u_colorBloom.rgb * overlayAlpha;

  vec3 colorWithOverlay = accumColor + accumAlpha * overlayColor;
  accumColor = mix(accumColor, colorWithOverlay, u_bloom);

  vec3 bgColor = u_colorBack.rgb * u_colorBack.a;

  vec3 color = accumColor + (1. - accumAlpha) * bgColor;
  float opacity = accumAlpha + (1. - accumAlpha) * u_colorBack.a;
  color = clamp(color, 0., 1.);
  opacity = clamp(opacity, 0., 1.);

  ${glsl.dither}

  fragColor = vec4(color, opacity);
}
`;

/** The properties people change in the god rays's controls and inspector. */
export const GOD_RAYS_PROPERTIES = {
    ...Shader.properties,
    speed: Shader.properties.speed.default(0.75),
    ...SIZING_PROPERTIES,
    offsetY: SIZING_PROPERTIES.offsetY.default(-0.55),
    colorBack: COLOR_BACK.default("#000000"),
    colorBloom: field
        .json(Color.schema)
        .labeled(t.context("label")`Bloom color`)
        .described(t`The color blended over the rays.`)
        .default("#0000ff"),
    colors: field
        .json(Palette.min(1).max(GOD_RAYS_MAX_COLOR_COUNT))
        .labeled(t.context("label")`Colors`)
        .described(t`The colors the effect blends, in order.`)
        .default(["#a600ff6e", "#6200fff0", "#ffffff", "#33fff5"]),
    density: field
        .number(schema.number().min(0).max(1))
        .labeled(t.context("label")`Density`)
        .described(t`How many rays there are.`)
        .default(0.3),
    spotty: field
        .number(schema.number().min(0).max(1))
        .labeled(t.context("label")`Spottiness`)
        .described(t`How short and spotty the rays are.`)
        .default(0.3),
    midIntensity: field
        .number(schema.number().min(0).max(1))
        .labeled(t.context("label")`Glow intensity`)
        .described(t`The brightness of the glow in the center.`)
        .default(0.4),
    midSize: field
        .number(schema.number().min(0).max(1))
        .labeled(t.context("label")`Glow size`)
        .described(t`The size of the glow in the center.`)
        .default(0.2),
    intensity: field
        .number(schema.number().min(0).max(1))
        .labeled(t.context("label")`Intensity`)
        .described(t`How strongly the rays show.`)
        .default(0.8),
    bloom: field
        .number(schema.number().min(0).max(1))
        .labeled(t.context("label")`Bloom`)
        .described(t`How strongly the bloom color blends over the rays.`)
        .default(0.4),
};

/** The options of the god rays, each with its value. */
export type GodRaysOptions = DeclaredProperties<typeof GOD_RAYS_PROPERTIES>;

/** Write god rays options as the fragment shader's uniforms. */
export function godRaysUniforms(options: GodRaysOptions): ShaderValues {
    return {
        u_colorBloom: options.colorBloom,
        u_colorBack: options.colorBack,
        u_colors: options.colors,
        u_colorsCount: options.colors.length,
        u_density: options.density,
        u_spotty: options.spotty,
        u_midIntensity: options.midIntensity,
        u_midSize: options.midSize,
        u_intensity: options.intensity,
        u_bloom: options.bloom,
        u_noiseTexture: NOISE_TEXTURE,
        ...sizingUniforms(options),
    };
}
