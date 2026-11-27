import { Palette } from "@destack/style";
import { t } from "@destack/locale";
import { field } from "@destack/object";
import { schema } from "@destack/schema";
import type { DeclaredProperties } from "@destack/view/declare";
import { Shader } from "../shader/shader.tsx";
import { SIZING_PROPERTIES, sizingUniforms } from "../shader/sizing.ts";
import { GRAIN_MIXER, GRAIN_OVERLAY, MIXING } from "../shader/option.ts";
import { glsl } from "../glsl/glsl.ts";
import { type ShaderValues } from "../shader/value.ts";

/** The most colors in the static mesh gradient. */
export const STATIC_MESH_GRADIENT_MAX_COLOR_COUNT = 10;

/** The fragment shader of the static mesh gradient. */
export const STATIC_MESH_GRADIENT_FRAGMENT = `#version 300 es
precision mediump float;

uniform vec4 u_colors[${STATIC_MESH_GRADIENT_MAX_COLOR_COUNT}];
uniform float u_colorsCount;

uniform float u_positions;
uniform float u_waveX;
uniform float u_waveXShift;
uniform float u_waveY;
uniform float u_waveYShift;
uniform float u_mixing;
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
  float b = .6 + mod(float(i), 3.) * .3;
  float c = .8 + mod(float(i + 1), 4.) * 0.25;

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

  float radius = smoothstep(0., 1., length(uv - .5));
  float center = 1. - radius;
  for (float i = 1.; i <= 2.; i++) {
    uv.x += u_waveX * center / i * cos(TWO_PI * u_waveXShift + i * 2. * smoothstep(.0, 1., uv.y));
    uv.y += u_waveY * center / i * cos(TWO_PI * u_waveYShift + i * 2. * smoothstep(.0, 1., uv.x));
  }

  vec3 color = vec3(0.);
  float opacity = 0.;
  float totalWeight = 0.;
  float positionSeed = 25. + .33 * u_positions;

  for (int i = 0; i < ${STATIC_MESH_GRADIENT_MAX_COLOR_COUNT}; i++) {
    if (i >= int(u_colorsCount)) break;

    vec2 pos = getPosition(i, positionSeed) + mixerGrain;
    float dist = length(uv - pos);

    vec3 colorFraction = u_colors[i].rgb * u_colors[i].a;
    float opacityFraction = u_colors[i].a;

    float mixing = pow(u_mixing, .7);
    float power = mix(2., 1., mixing);
    dist = pow(dist, power);

    float w = 1. / (dist + 1e-3);
    float baseSharpness = mix(.0, 8., clamp(w, 0., 1.));
    float sharpness = mix(baseSharpness, 1., mixing);
    w = pow(w, sharpness);
    color += colorFraction * w;
    opacity += opacityFraction * w;
    totalWeight += w;
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

/** The properties people change in the static mesh gradient's controls and inspector. */
export const STATIC_MESH_GRADIENT_PROPERTIES = {
    ...Shader.properties,
    ...SIZING_PROPERTIES,
    rotation: SIZING_PROPERTIES.rotation.default(270),
    colors: field
        .json(Palette.min(1).max(STATIC_MESH_GRADIENT_MAX_COLOR_COUNT))
        .labeled(t.context("label")`Colors`)
        .described(t`The colors the effect blends, in order.`)
        .default(["#ffad0a", "#6200ff", "#e2a3ff", "#ff99fd"]),
    positions: field
        .number(schema.number().min(0).max(100))
        .labeled(t.context("label")`Positions`)
        .described(t`The seed that places the color spots.`)
        .default(2),
    waveX: field
        .number(schema.number().min(0).max(1))
        .labeled(t.context("label")`Horizontal wave`)
        .described(t`How strongly a wave bends the colors across.`)
        .default(1.0),
    waveXShift: field
        .number(schema.number().min(0).max(1))
        .labeled(t.context("label")`Horizontal wave shift`)
        .described(t`The phase of the wave across.`)
        .default(0.6),
    waveY: field
        .number(schema.number().min(0).max(1))
        .labeled(t.context("label")`Vertical wave`)
        .described(t`How strongly a wave bends the colors down.`)
        .default(1.0),
    waveYShift: field
        .number(schema.number().min(0).max(1))
        .labeled(t.context("label")`Vertical wave shift`)
        .described(t`The phase of the wave down.`)
        .default(0.21),
    mixing: MIXING.default(0.93),
    grainMixer: GRAIN_MIXER.default(0.0),
    grainOverlay: GRAIN_OVERLAY.default(0.0),
};

/** The options of the static mesh gradient, each with its value. */
export type StaticMeshGradientOptions = DeclaredProperties<typeof STATIC_MESH_GRADIENT_PROPERTIES>;

/** Write static mesh gradient options as the fragment shader's uniforms. */
export function staticMeshGradientUniforms(options: StaticMeshGradientOptions): ShaderValues {
    return {
        u_colors: options.colors,
        u_colorsCount: options.colors.length,
        u_positions: options.positions,
        u_waveX: options.waveX,
        u_waveXShift: options.waveXShift,
        u_waveY: options.waveY,
        u_waveYShift: options.waveYShift,
        u_mixing: options.mixing,
        u_grainMixer: options.grainMixer,
        u_grainOverlay: options.grainOverlay,
        ...sizingUniforms(options),
    };
}
