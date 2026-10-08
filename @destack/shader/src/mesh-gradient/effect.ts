import { glsl } from "../glsl/glsl.ts";
import { OBJECT_SIZING, type Sizing, sizingUniforms } from "../shader/sizing.ts";
import { type ShaderColor, type ShaderValues } from "../shader/value.ts";
import type { ShaderProperties } from "../shader/shader.tsx";

/** The most colors in the mesh gradient. */
export const MESH_GRADIENT_MAX_COLOR_COUNT = 10;

/** The fragment shader of the mesh gradient. */
export const MESH_GRADIENT_FRAGMENT = `#version 300 es
precision mediump float;

uniform float u_time;

uniform vec4 u_colors[${MESH_GRADIENT_MAX_COLOR_COUNT}];
uniform float u_colorsCount;

uniform float u_distortion;
uniform float u_swirl;
uniform float u_grainMixer;
uniform float u_grainOverlay;

in vec2 v_objectUV;
out vec4 fragColor;

${glsl.pi}
${glsl.rotate}
${glsl.hash21}

float valueNoise(vec2 st) {
  vec2 i = floor(st);
  vec2 f = fract(st);
  float a = hash21(i);
  float b = hash21(i + vec2(1.0, 0.0));
  float c = hash21(i + vec2(0.0, 1.0));
  float d = hash21(i + vec2(1.0, 1.0));
  vec2 u = f * f * (3.0 - 2.0 * f);
  float x1 = mix(a, b, u.x);
  float x2 = mix(c, d, u.x);
  return mix(x1, x2, u.y);
}

float noise(vec2 n, vec2 seedOffset) {
  return valueNoise(n + seedOffset);
}

vec2 getPosition(int i, float t) {
  float a = float(i) * .37;
  float b = .6 + fract(float(i) / 3.) * .9;
  float c = .8 + fract(float(i + 1) / 4.);

  float x = sin(t * b + a);
  float y = cos(t * c + a * 1.5);

  return .5 + .5 * vec2(x, y);
}

void main() {
  vec2 uv = v_objectUV;
  uv += .5;
  vec2 grainUV = uv * 1000.;

  float mixerGrain = 0.;
  if (u_grainMixer > 0.) {
    mixerGrain = .4 * u_grainMixer * (noise(grainUV, vec2(0.)) - .5);
  }

  const float firstFrameOffset = 41.5;
  float t = .5 * (u_time + firstFrameOffset);

  float radius = smoothstep(0., 1., length(uv - .5));
  float center = 1. - radius;
  for (float i = 1.; i <= 2.; i++) {
    uv.x += u_distortion * center / i * sin(t + i * .4 * smoothstep(.0, 1., uv.y)) * cos(.2 * t + i * 2.4 * smoothstep(.0, 1., uv.y));
    uv.y += u_distortion * center / i * cos(t + i * 2. * smoothstep(.0, 1., uv.x));
  }

  vec2 uvRotated = uv;
  uvRotated -= vec2(.5);
  float angle = 3. * u_swirl * radius;
  uvRotated = rotate(uvRotated, -angle);
  uvRotated += vec2(.5);

  vec3 color = vec3(0.);
  float opacity = 0.;
  float totalWeight = 0.;

  for (int i = 0; i < ${MESH_GRADIENT_MAX_COLOR_COUNT}; i++) {
    if (i >= int(u_colorsCount)) break;

    vec2 pos = getPosition(i, t) + mixerGrain;
    vec3 colorFraction = u_colors[i].rgb * u_colors[i].a;
    float opacityFraction = u_colors[i].a;

    float dist = length(uvRotated - pos);

    dist = pow(dist, 3.5);
    float weight = 1. / (dist + 1e-3);
    color += colorFraction * weight;
    opacity += opacityFraction * weight;
    totalWeight += weight;
  }

  color /= max(1e-4, totalWeight);
  opacity /= max(1e-4, totalWeight);

  if (u_grainOverlay > 0.) {
    float grainOverlay = valueNoise(rotate(grainUV, 1.) + vec2(3.));
    grainOverlay = mix(grainOverlay, valueNoise(rotate(grainUV, 2.) + vec2(-1.)), .5);
    grainOverlay = pow(grainOverlay, 1.3);

    float grainOverlayV = grainOverlay * 2. - 1.;
    vec3 grainOverlayColor = vec3(step(0., grainOverlayV));
    float grainOverlayStrength = u_grainOverlay * abs(grainOverlayV);
    grainOverlayStrength = pow(grainOverlayStrength, .8);
    color = mix(color, grainOverlayColor, .35 * grainOverlayStrength);

    opacity += .5 * grainOverlayStrength;
  }
  opacity = clamp(opacity, 0., 1.);

  fragColor = vec4(color, opacity);
}
`;

/** The options of the mesh gradient, beside its sizing. */
export interface MeshGradientOptions extends Sizing, Pick<ShaderProperties, "speed" | "frame"> {
    /** Up to 10 color spots. */
    readonly colors?: readonly ShaderColor[];
    /** The power of organic noise distortion, from 0 to 1. */
    readonly distortion?: number;
    /** The power of vortex distortion, from 0 to 1. */
    readonly swirl?: number;
    /** The strength of grain distortion applied to shape edges, from 0 to 1. */
    readonly grainMixer?: number;
    /** The post-processing black/white grain overlay, from 0 to 1. */
    readonly grainOverlay?: number;
}

/** The properties of the mesh gradient, the shader's included. */
export interface MeshGradientProperties
    extends MeshGradientOptions, Omit<ShaderProperties, "fragment" | "uniforms"> {}

/** The default mesh gradient. */
export const MESH_GRADIENT_DEFAULTS: Required<MeshGradientOptions> = {
    ...OBJECT_SIZING,
    speed: 1,
    frame: 0,
    colors: ["#e0eaff", "#241d9a", "#f75092", "#9f50d3"],
    distortion: 0.8,
    swirl: 0.1,
    grainMixer: 0,
    grainOverlay: 0,
};

/** Write mesh gradient options as the fragment shader's uniforms. */
export function meshGradientUniforms(options: Required<MeshGradientOptions>): ShaderValues {
    return {
        u_colors: options.colors,
        u_colorsCount: options.colors.length,
        u_distortion: options.distortion,
        u_swirl: options.swirl,
        u_grainMixer: options.grainMixer,
        u_grainOverlay: options.grainOverlay,
        ...sizingUniforms(options),
    };
}
