import { Color, Palette } from "@destack/style";
import { t } from "@destack/locale";
import { field } from "@destack/object";
import { schema } from "@destack/schema";
import type { DeclaredProperties } from "@destack/view/declare";
import { Shader } from "../shader/shader.tsx";
import { PATTERN_SIZING_PROPERTIES, sizingUniforms } from "../shader/sizing.ts";
import { glsl } from "../glsl/glsl.ts";
import { NOISE_TEXTURE } from "../shader/noise.ts";
import { type ShaderValues } from "../shader/value.ts";

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

/** The properties people change in the voronoi's controls and inspector. */
export const VORONOI_PROPERTIES = {
    ...Shader.properties,
    speed: Shader.properties.speed.default(0.5),
    ...PATTERN_SIZING_PROPERTIES,
    scale: PATTERN_SIZING_PROPERTIES.scale.default(0.5),
    colors: field
        .json(Palette.min(1).max(VORONOI_MAX_COLOR_COUNT))
        .labeled(t.context("label")`Colors`)
        .described(t`The colors the effect blends, in order.`)
        .default(["#ff8247", "#ffe53d"]),
    stepsPerColor: field
        .integer(schema.number().int().min(1).max(3))
        .labeled(t.context("label")`Steps per color`)
        .described(t`How many colors lie between each two given ones.`)
        .default(3),
    colorGlow: field
        .json(Color.schema)
        .labeled(t.context("label")`Glow color`)
        .described(t`The color of the glow inside each cell.`)
        .default("#ffffff"),
    colorGap: field
        .json(Color.schema)
        .labeled(t.context("label")`Gap color`)
        .described(t`The color of the gaps between cells.`)
        .default("#2e0000"),
    distortion: field
        .number(schema.number().min(0).max(0.5))
        .labeled(t.context("label")`Distortion`)
        .described(t`How far noise moves the center of each cell.`)
        .default(0.4),
    gap: field
        .number(schema.number().min(0).max(0.1))
        .labeled(t.context("label")`Gap`)
        .described(t`The width of the gaps between cells.`)
        .default(0.04),
    glow: field
        .number(schema.number().min(0).max(1))
        .labeled(t.context("label")`Glow`)
        .described(t`How strongly each cell glows inside.`)
        .default(0),
};

/** The options of the voronoi, each with its value. */
export type VoronoiOptions = DeclaredProperties<typeof VORONOI_PROPERTIES>;

/** Write voronoi options as the fragment shader's uniforms. */
export function voronoiUniforms(options: VoronoiOptions): ShaderValues {
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
