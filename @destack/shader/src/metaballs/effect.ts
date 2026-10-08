import { glsl } from "../glsl/glsl.ts";
import { NOISE_TEXTURE } from "../shader/noise.ts";
import { OBJECT_SIZING, type Sizing, sizingUniforms } from "../shader/sizing.ts";
import { type ShaderColor, type ShaderValues } from "../shader/value.ts";
import type { ShaderProperties } from "../shader/shader.tsx";

/** The most colors in the metaballs. */
export const METABALLS_MAX_COLOR_COUNT = 8;

/** The most balls in the metaballs. */
export const METABALLS_MAX_BALLS_COUNT = 20;

/** The fragment shader of the metaballs. */
export const METABALLS_FRAGMENT = `#version 300 es
precision mediump float;

uniform float u_time;

uniform sampler2D u_noiseTexture;

uniform vec4 u_colorBack;
uniform vec4 u_colors[${METABALLS_MAX_COLOR_COUNT}];
uniform float u_colorsCount;
uniform float u_size;
uniform float u_count;

in vec2 v_objectUV;

out vec4 fragColor;

${glsl.pi}
${glsl.randomR}
float noise(float x) {
  float i = floor(x);
  float f = fract(x);
  float u = f * f * (3.0 - 2.0 * f);
  vec2 p0 = vec2(i, 0.0);
  vec2 p1 = vec2(i + 1.0, 0.0);
  return mix(randomR(p0), randomR(p1), u);
}

float getBallShape(vec2 uv, vec2 c, float p) {
  float s = .5 * length(uv - c);
  s = 1. - clamp(s, 0., 1.);
  s = pow(s, p);
  return s;
}

void main() {
  vec2 shape_uv = v_objectUV;

  shape_uv += .5;

  const float firstFrameOffset = 2503.4;
  float t = .2 * (u_time + firstFrameOffset);

  vec3 totalColor = vec3(0.);
  float totalShape = 0.;
  float totalOpacity = 0.;

  for (int i = 0; i < ${METABALLS_MAX_BALLS_COUNT}; i++) {
    if (i >= int(ceil(u_count))) break;

    float idxFract = float(i) / float(${METABALLS_MAX_BALLS_COUNT});
    float angle = TWO_PI * idxFract;

    float speed = 1. - .2 * idxFract;
    float noiseX = noise(angle * 10. + float(i) + t * speed);
    float noiseY = noise(angle * 20. + float(i) - t * speed);

    vec2 pos = vec2(.5) + 1e-4 + .9 * (vec2(noiseX, noiseY) - .5);

    int safeIndex = i % int(u_colorsCount + 0.5);
    vec4 ballColor = u_colors[safeIndex];
    ballColor.rgb *= ballColor.a;

    float sizeFrac = 1.;
    if (float(i) > floor(u_count - 1.)) {
      sizeFrac *= fract(u_count);
    }

    float shape = getBallShape(shape_uv, pos, 45. - 30. * u_size * sizeFrac);
    shape *= pow(u_size, .2);
    shape = smoothstep(0., 1., shape);

    totalColor += ballColor.rgb * shape;
    totalShape += shape;
    totalOpacity += ballColor.a * shape;
  }

  totalColor /= max(totalShape, 1e-4);
  totalOpacity /= max(totalShape, 1e-4);

  float edge_width = fwidth(totalShape);
  float finalShape = smoothstep(.4, .4 + edge_width, totalShape);

  vec3 color = totalColor * finalShape;
  float opacity = totalOpacity * finalShape;

  vec3 bgColor = u_colorBack.rgb * u_colorBack.a;
  color = color + bgColor * (1. - opacity);
  opacity = opacity + u_colorBack.a * (1. - opacity);

  ${glsl.dither}

  fragColor = vec4(color, opacity);
}
`;

/** The options of the metaballs, beside its sizing. */
export interface MetaballsOptions extends Sizing, Pick<ShaderProperties, "speed" | "frame"> {
    /** The background color. */
    readonly colorBack?: ShaderColor;
    /** Up to 8 base colors, applied to the balls in order. */
    readonly colors?: readonly ShaderColor[];
    /** The number of balls, fractional values shrink the last ball, from 1 to 20. */
    readonly count?: number;
    /** The size of the balls, from 0 to 1. */
    readonly size?: number;
}

/** The properties of the metaballs, the shader's included. */
export interface MetaballsProperties
    extends MetaballsOptions, Omit<ShaderProperties, "fragment" | "uniforms"> {}

/** The default metaballs. */
export const METABALLS_DEFAULTS: Required<MetaballsOptions> = {
    ...OBJECT_SIZING,
    scale: 1,
    speed: 1,
    frame: 0,
    colorBack: "#000000",
    colors: ["#6e33cc", "#ff5500", "#ffc105", "#ffc800", "#f585ff"],
    count: 10,
    size: 0.83,
};

/** Write metaballs options as the fragment shader's uniforms. */
export function metaballsUniforms(options: Required<MetaballsOptions>): ShaderValues {
    return {
        u_colorBack: options.colorBack,
        u_colors: options.colors,
        u_colorsCount: options.colors.length,
        u_size: options.size,
        u_count: options.count,
        u_noiseTexture: NOISE_TEXTURE,
        ...sizingUniforms(options),
    };
}
