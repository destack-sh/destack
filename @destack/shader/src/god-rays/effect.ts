import { glsl } from "../glsl/glsl.ts";
import { NOISE_TEXTURE } from "../shader/noise.ts";
import { OBJECT_SIZING, type Sizing, sizingUniforms } from "../shader/sizing.ts";
import { type ShaderColor, type ShaderValues } from "../shader/value.ts";
import type { ShaderProperties } from "../shader/shader.tsx";

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

/** The options of the god rays, beside its sizing. */
export interface GodRaysOptions extends Sizing, Pick<ShaderProperties, "speed" | "frame"> {
    /** The background color. */
    readonly colorBack?: ShaderColor;
    /** The color overlay blended with the rays, needs bloom > 0. */
    readonly colorBloom?: ShaderColor;
    /** Up to 5 ray colors. */
    readonly colors?: readonly ShaderColor[];
    /** The number of rays, from 0 to 1. */
    readonly density?: number;
    /** The length of the rays, higher = more spots/shorter rays, from 0 to 1. */
    readonly spotty?: number;
    /** The brightness/intensity of the central glow, needs midSize > 0, from 0 to 1. */
    readonly midIntensity?: number;
    /** The size of the circular glow shape in the center, needs midIntensity > 0, from 0 to 1. */
    readonly midSize?: number;
    /** The visibility/strength of the rays, from 0 to 1. */
    readonly intensity?: number;
    /** The strength of the bloom/overlay effect, 0 = alpha blend, 1 = additive blend, from 0 to 1. */
    readonly bloom?: number;
}

/** The properties of the god rays, the shader's included. */
export interface GodRaysProperties
    extends GodRaysOptions, Omit<ShaderProperties, "fragment" | "uniforms"> {}

/** The default god rays. */
export const GOD_RAYS_DEFAULTS: Required<GodRaysOptions> = {
    ...OBJECT_SIZING,
    offsetX: 0,
    offsetY: -0.55,
    colorBack: "#000000",
    colorBloom: "#0000ff",
    colors: ["#a600ff6e", "#6200fff0", "#ffffff", "#33fff5"],
    density: 0.3,
    spotty: 0.3,
    midIntensity: 0.4,
    midSize: 0.2,
    intensity: 0.8,
    bloom: 0.4,
    speed: 0.75,
    frame: 0,
};

/** Write god rays options as the fragment shader's uniforms. */
export function godRaysUniforms(options: Required<GodRaysOptions>): ShaderValues {
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
