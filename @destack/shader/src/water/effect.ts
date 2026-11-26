import { t } from "@destack/locale";
import { field } from "@destack/object";
import { schema } from "@destack/schema";
import type { DeclaredProperties } from "@destack/view/declare";
import { Shader } from "../shader/shader.tsx";
import { SIZING_PROPERTIES, sizingUniforms } from "../shader/sizing.ts";
import { COLOR_BACK, COLOR_HIGHLIGHT } from "../shader/option.ts";
import { glsl } from "../glsl/glsl.ts";
import { TRANSPARENT_PIXEL, type ShaderValues } from "../shader/value.ts";

/** The fragment shader of the water. */
export const WATER_FRAGMENT = `#version 300 es
precision mediump float;

uniform float u_time;

uniform vec4 u_colorBack;
uniform vec4 u_colorHighlight;

uniform sampler2D u_image;
uniform float u_imageAspectRatio;

uniform float u_size;
uniform float u_highlights;
uniform float u_layering;
uniform float u_edges;
uniform float u_caustic;
uniform float u_waves;

in vec2 v_imageUV;

out vec4 fragColor;

${glsl.pi}
${glsl.rotate}
${glsl.simplex}

float getUvFrame(vec2 uv) {
  float aax = 2. * fwidth(uv.x);
  float aay = 2. * fwidth(uv.y);

  float left   = smoothstep(0., aax, uv.x);
  float right = 1.0 - smoothstep(1. - aax, 1., uv.x);
  float bottom = smoothstep(0., aay, uv.y);
  float top = 1.0 - smoothstep(1. - aay, 1., uv.y);

  return left * right * bottom * top;
}

mat2 rotate2D(float r) {
  return mat2(cos(r), sin(r), -sin(r), cos(r));
}

float getCausticNoise(vec2 uv, float t, float scale) {
  vec2 n = vec2(.1);
  vec2 N = vec2(.1);
  mat2 m = rotate2D(.5);
  for (int j = 0; j < 6; j++) {
    uv *= m;
    n *= m;
    vec2 q = uv * scale + float(j) + n + (.5 + .5 * float(j)) * (mod(float(j), 2.) - 1.) * t;
    n += sin(q);
    N += cos(q) / scale;
    scale *= 1.1;
  }
  return (N.x + N.y + 1.);
}

void main() {
  vec2 imageUV = v_imageUV;
  vec2 patternUV = v_imageUV - .5;
  patternUV = (patternUV * vec2(u_imageAspectRatio, 1.));
  patternUV /= (.01 + .09 * u_size);

  float t = u_time;

  float wavesNoise = snoise((.3 + .1 * sin(t)) * .1 * patternUV + vec2(0., .4 * t));

  float causticNoise = getCausticNoise(patternUV + u_waves * vec2(1., -1.) * wavesNoise, 2. * t, 1.5);

  if (u_layering > 0.) {
    causticNoise += u_layering * getCausticNoise(patternUV + 2. * u_waves * vec2(1., -1.) * wavesNoise, 1.5 * t, 2.);
  }
  causticNoise = causticNoise * causticNoise;

  float edgesDistortion = smoothstep(0., .1, imageUV.x);
  edgesDistortion *= smoothstep(0., .1, imageUV.y);
  edgesDistortion *= (smoothstep(1., 1.1, imageUV.x) + (1.0 - smoothstep(.8, .95, imageUV.x)));
  edgesDistortion *= (1.0 - smoothstep(.9, 1., imageUV.y));
  edgesDistortion = mix(edgesDistortion, 1., u_edges);

  float causticNoiseDistortion = .02 * causticNoise * edgesDistortion;

  float wavesDistortion = .1 * u_waves * wavesNoise;

  imageUV += vec2(wavesDistortion, -wavesDistortion);
  imageUV += (u_caustic * causticNoiseDistortion);

  float frame = getUvFrame(imageUV);

  vec4 image = texture(u_image, imageUV);
  vec4 backColor = u_colorBack;
  backColor.rgb *= backColor.a;

  vec3 color = mix(backColor.rgb, image.rgb, image.a * frame);
  float opacity = backColor.a + image.a * frame;

  causticNoise = max(-.2, causticNoise);

  float hightlight = .025 * u_highlights * causticNoise;
  hightlight *= u_colorHighlight.a;
  color = mix(color, u_colorHighlight.rgb, .05 * u_highlights * causticNoise * u_colorHighlight.a);
  opacity += hightlight;

  color += hightlight * (.5 + .5 * wavesNoise);
  opacity += hightlight * (.5 + .5 * wavesNoise);

  opacity = clamp(opacity, 0., 1.);

  fragColor = vec4(color, opacity);
}
`;

/** The properties people change in the water's controls and inspector. */
export const WATER_PROPERTIES = {
    ...Shader.properties,
    speed: Shader.properties.speed.default(1),
    ...SIZING_PROPERTIES,
    scale: SIZING_PROPERTIES.scale.default(0.8),
    colorBack: COLOR_BACK.default("#909090"),
    colorHighlight: COLOR_HIGHLIGHT.default("#ffffff"),
    highlights: field
        .number(schema.number().min(0).max(1))
        .labeled(t.context("label")`Highlights`)
        .described(t`The highlights along the light patterns.`)
        .default(0.07),
    layering: field
        .number(schema.number().min(0).max(1))
        .labeled(t.context("label")`Layering`)
        .described(t`How strongly a second layer of light bends the image.`)
        .default(0.5),
    edges: field
        .number(schema.number().min(0).max(1))
        .labeled(t.context("label")`Edges`)
        .described(t`How much the water bends the image's edges.`)
        .default(0.8),
    waves: field
        .number(schema.number().min(0).max(1))
        .labeled(t.context("label")`Waves`)
        .described(t`The extra bending of noise.`)
        .default(0.3),
    caustic: field
        .number(schema.number().min(0).max(1))
        .labeled(t.context("label")`Caustic`)
        .described(t`How strongly the light patterns bend the image.`)
        .default(0.1),
    size: field
        .number(schema.number().min(0.01).max(7))
        .labeled(t.context("label")`Size`)
        .described(t`The size of the pattern within the image.`)
        .default(1),
};

/** The options of the water, each with its value. */
export type WaterOptions = DeclaredProperties<typeof WATER_PROPERTIES> & {
    /** The image it draws over, none for its background alone. */
    readonly image?: string | HTMLImageElement | null;
};

/** Write water options as the fragment shader's uniforms. */
export function waterUniforms(options: WaterOptions): ShaderValues {
    return {
        u_image: { image: options.image ?? TRANSPARENT_PIXEL },
        u_colorBack: options.colorBack,
        u_colorHighlight: options.colorHighlight,
        u_highlights: options.highlights,
        u_layering: options.layering,
        u_waves: options.waves,
        u_edges: options.edges,
        u_caustic: options.caustic,
        u_size: options.size,
        ...sizingUniforms(options),
    };
}
