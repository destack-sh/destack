import { glsl } from "../glsl/glsl.ts";
import { NOISE_TEXTURE } from "../shader/noise.ts";
import { PATTERN_SIZING, type Sizing, sizingUniforms } from "../shader/sizing.ts";
import { type ShaderColor, type ShaderValues } from "../shader/value.ts";
import type { ShaderProperties } from "../shader/shader.tsx";

/** The most colors in the voronoi. */
export const VORONOI_MAX_COLOR_COUNT = 5;

/** The fragment shader of the voronoi. */
export const VORONOI_FRAGMENT = `#version 300 es
precision mediump float;

uniform float u_time;

uniform float u_scale;

uniform sampler2D u_noiseTexture;

uniform vec4 u_colors[${VORONOI_MAX_COLOR_COUNT}];
uniform float u_colorsCount;

uniform float u_stepsPerColor;
uniform vec4 u_colorGlow;
uniform vec4 u_colorGap;
uniform float u_distortion;
uniform float u_gap;
uniform float u_glow;

in vec2 v_patternUV;

out vec4 fragColor;

${glsl.pi}
${glsl.randomGB}

vec4 voronoi(vec2 x, float t) {
  vec2 ip = floor(x);
  vec2 fp = fract(x);

  vec2 mg, mr;
  float md = 8.;
  float rand = 0.;

  for (int j = -1; j <= 1; j++) {
    for (int i = -1; i <= 1; i++) {
      vec2 g = vec2(float(i), float(j));
      vec2 o = randomGB(ip + g);
      float raw_hash = o.x;
      o = .5 + u_distortion * sin(t + TWO_PI * o);
      vec2 r = g + o - fp;
      float d = dot(r, r);

      if (d < md) {
        md = d;
        mr = r;
        mg = g;
        rand = raw_hash;
      }
    }
  }

  md = 8.;
  for (int j = -2; j <= 2; j++) {
    for (int i = -2; i <= 2; i++) {
      vec2 g = mg + vec2(float(i), float(j));
      vec2 o = randomGB(ip + g);
      o = .5 + u_distortion * sin(t + TWO_PI * o);
      vec2 r = g + o - fp;
      if (dot(mr - r, mr - r) > .00001) {
        md = min(md, dot(.5 * (mr + r), normalize(r - mr)));
      }
    }
  }

  return vec4(md, mr, rand);
}

void main() {
  vec2 shape_uv = v_patternUV;
  shape_uv *= 1.25;

  float t = u_time;

  vec4 voronoiRes = voronoi(shape_uv, t);

  float shape = clamp(voronoiRes.w, 0., 1.);
  float mixer = shape * (u_colorsCount - 1.);
  mixer = (shape - .5 / u_colorsCount) * u_colorsCount;
  float steps = max(1., u_stepsPerColor);

  vec4 gradient = u_colors[0];
  gradient.rgb *= gradient.a;
  for (int i = 1; i < ${VORONOI_MAX_COLOR_COUNT}; i++) {
    if (i >= int(u_colorsCount)) break;
    float localT = clamp(mixer - float(i - 1), 0.0, 1.0);
    localT = round(localT * steps) / steps;
    vec4 c = u_colors[i];
    c.rgb *= c.a;
    gradient = mix(gradient, c, localT);
  }

  if ((mixer < 0.) || (mixer > (u_colorsCount - 1.))) {
    float localT = mixer + 1.;
    if (mixer > (u_colorsCount - 1.)) {
      localT = mixer - (u_colorsCount - 1.);
    }
    localT = round(localT * steps) / steps;
    vec4 cFst = u_colors[0];
    cFst.rgb *= cFst.a;
    vec4 cLast = u_colors[int(u_colorsCount - 1.)];
    cLast.rgb *= cLast.a;
    gradient = mix(cLast, cFst, localT);
  }

  vec3 cellColor = gradient.rgb;
  float cellOpacity = gradient.a;

  float glows = length(voronoiRes.yz * u_glow);
  glows = pow(glows, 1.5);

  vec3 color = mix(cellColor, u_colorGlow.rgb * u_colorGlow.a, u_colorGlow.a * glows);
  float opacity = cellOpacity + u_colorGlow.a * glows;

  float edge = voronoiRes.x;
  float smoothEdge = .02 / (2. * u_scale) * (1. + .5 * u_gap);
  edge = smoothstep(u_gap - smoothEdge, u_gap + smoothEdge, edge);

  color = mix(u_colorGap.rgb * u_colorGap.a, color, edge);
  opacity = mix(u_colorGap.a, opacity, edge);

  fragColor = vec4(color, opacity);
}
`;

/** The options of the voronoi, beside its sizing. */
export interface VoronoiOptions extends Sizing, Pick<ShaderProperties, "speed" | "frame"> {
    /** Up to 5 base cell colors. */
    readonly colors?: readonly ShaderColor[];
    /** The number of extra colors between base colors, 1 = N colors, 2 = 2×N, etc., needs 2+ colors, from 1 to 3. */
    readonly stepsPerColor?: number;
    /** The color tint for radial inner shadow inside cells, needs glow > 0. */
    readonly colorGlow?: ShaderColor;
    /** The color used for cell borders/gaps. */
    readonly colorGap?: ShaderColor;
    /** The strength of noise-driven displacement of cell centers, from 0 to 0.5. */
    readonly distortion?: number;
    /** The width of the border/gap between cells, from 0 to 0.1. */
    readonly gap?: number;
    /** The strength of the radial inner shadow inside cells, needs colorGlow alpha > 0, from 0 to 1. */
    readonly glow?: number;
}

/** The properties of the voronoi, the shader's included. */
export interface VoronoiProperties
    extends VoronoiOptions, Omit<ShaderProperties, "fragment" | "uniforms"> {}

/** The default voronoi. */
export const VORONOI_DEFAULTS: Required<VoronoiOptions> = {
    ...PATTERN_SIZING,
    speed: 0.5,
    frame: 0,
    colors: ["#ff8247", "#ffe53d"],
    stepsPerColor: 3,
    colorGlow: "#ffffff",
    colorGap: "#2e0000",
    distortion: 0.4,
    gap: 0.04,
    glow: 0,
    scale: 0.5,
};

/** Write voronoi options as the fragment shader's uniforms. */
export function voronoiUniforms(options: Required<VoronoiOptions>): ShaderValues {
    return {
        u_colors: options.colors,
        u_colorsCount: options.colors.length,
        u_stepsPerColor: options.stepsPerColor,
        u_colorGlow: options.colorGlow,
        u_colorGap: options.colorGap,
        u_distortion: options.distortion,
        u_gap: options.gap,
        u_glow: options.glow,
        u_noiseTexture: NOISE_TEXTURE,
        ...sizingUniforms(options),
    };
}
