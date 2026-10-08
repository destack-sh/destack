import { glsl } from "../glsl/glsl.ts";
import { PATTERN_SIZING, type Sizing, sizingUniforms } from "../shader/sizing.ts";
import { type ShaderColor, type ShaderValues } from "../shader/value.ts";
import type { ShaderProperties } from "../shader/shader.tsx";

/** A shape of the dot grid. */
export type DotGridShape = "circle" | "diamond" | "square" | "triangle";

/** The number the fragment shader reads for each shape. */
export const DOT_GRID_SHAPES: Readonly<Record<DotGridShape, number>> = {
    circle: 0,
    diamond: 1,
    square: 2,
    triangle: 3,
};

/** The fragment shader of the dot grid. */
export const DOT_GRID_FRAGMENT = `#version 300 es
precision mediump float;

uniform vec4 u_colorBack;
uniform vec4 u_colorFill;
uniform vec4 u_colorStroke;
uniform float u_dotSize;
uniform float u_gapX;
uniform float u_gapY;
uniform float u_strokeWidth;
uniform float u_sizeRange;
uniform float u_opacityRange;
uniform float u_shape;

in vec2 v_patternUV;

out vec4 fragColor;

${glsl.pi}
${glsl.simplex}

float polygon(vec2 p, float N, float rot) {
  float a = atan(p.x, p.y) + rot;
  float r = TWO_PI / float(N);

  return cos(floor(.5 + a / r) * r - a) * length(p);
}

void main() {

  // x100 is a default multiplier between vertex and fragmant shaders
  // we use it to avoid UV presision issues
  vec2 shape_uv = 100. * v_patternUV;

  vec2 gap = max(abs(vec2(u_gapX, u_gapY)), vec2(1e-6));
  vec2 grid = fract(shape_uv / gap) + 1e-4;
  vec2 grid_idx = floor(shape_uv / gap);
  float sizeRandomizer = .5 + .8 * snoise(2. * vec2(grid_idx.x * 100., grid_idx.y));
  float opacity_randomizer = .5 + .7 * snoise(2. * vec2(grid_idx.y, grid_idx.x));

  vec2 center = vec2(0.5) - 1e-3;
  vec2 p = (grid - center) * vec2(u_gapX, u_gapY);

  float baseSize = u_dotSize * (1. - sizeRandomizer * u_sizeRange);
  float strokeWidth = u_strokeWidth * (1. - sizeRandomizer * u_sizeRange);

  float dist;
  if (u_shape < 0.5) {
    // Circle
    dist = length(p);
  } else if (u_shape < 1.5) {
    // Diamond
    strokeWidth *= 1.5;
    dist = polygon(1.5 * p, 4., .25 * PI);
  } else if (u_shape < 2.5) {
    // Square
    dist = polygon(1.03 * p, 4., 1e-3);
  } else {
    // Triangle
    strokeWidth *= 1.5;
    p = p * 2. - 1.;
    p *= .9;
    p.y = 1. - p.y;
    p.y -= .75 * baseSize;
    dist = polygon(p, 3., 1e-3);
  }

  float edgeWidth = fwidth(dist);
  float shapeOuter = 1. - smoothstep(baseSize - edgeWidth, baseSize + edgeWidth, dist - strokeWidth);
  float shapeInner = 1. - smoothstep(baseSize - edgeWidth, baseSize + edgeWidth, dist);
  float stroke = shapeOuter - shapeInner;

  float dotOpacity = max(0., 1. - opacity_randomizer * u_opacityRange);
  stroke *= dotOpacity;
  shapeInner *= dotOpacity;

  stroke *= u_colorStroke.a;
  shapeInner *= u_colorFill.a;

  vec3 color = vec3(0.);
  color += stroke * u_colorStroke.rgb;
  color += shapeInner * u_colorFill.rgb;
  color += (1. - shapeInner - stroke) * u_colorBack.rgb * u_colorBack.a;

  float opacity = 0.;
  opacity += stroke;
  opacity += shapeInner;
  opacity += (1. - opacity) * u_colorBack.a;

  fragColor = vec4(color, opacity);
}
`;

/** The options of the dot grid, beside its sizing. */
export interface DotGridOptions extends Sizing, Pick<ShaderProperties, "speed" | "frame"> {
    /** The background color. */
    readonly colorBack?: ShaderColor;
    /** The shape fill color. */
    readonly colorFill?: ShaderColor;
    /** The shape stroke color, needs strokeWidth > 0. */
    readonly colorStroke?: ShaderColor;
    /** The size. */
    readonly size?: number;
    /** The pattern horizontal spacing in pixels, from 2 to 500. */
    readonly gapX?: number;
    /** The pattern vertical spacing in pixels, from 2 to 500. */
    readonly gapY?: number;
    /** The outline stroke width in pixels, needs colorStroke alpha > 0, from 0 to 50. */
    readonly strokeWidth?: number;
    /** The random variation in shape size, 0 = uniform, higher = random up to base size, from 0 to 1. */
    readonly sizeRange?: number;
    /** The random variation in shape opacity, 0 = opaque, higher = semi-transparent, from 0 to 1. */
    readonly opacityRange?: number;
    /** The shape type. */
    readonly shape?: DotGridShape;
}

/** The properties of the dot grid, the shader's included. */
export interface DotGridProperties
    extends DotGridOptions, Omit<ShaderProperties, "fragmentShader" | "uniforms"> {}

/** The default dot grid. */
export const DOT_GRID_DEFAULTS: Required<DotGridOptions> = {
    speed: 0,
    frame: 0,
    ...PATTERN_SIZING,
    colorBack: "#000000",
    colorFill: "#ffffff",
    colorStroke: "#ffaa00",
    size: 2,
    gapX: 32,
    gapY: 32,
    strokeWidth: 0,
    sizeRange: 0,
    opacityRange: 0,
    shape: "circle",
};

/** Write dot grid options as the fragment shader's uniforms. */
export function dotGridUniforms(options: Required<DotGridOptions>): ShaderValues {
    return {
        u_colorBack: options.colorBack,
        u_colorFill: options.colorFill,
        u_colorStroke: options.colorStroke,
        u_dotSize: options.size,
        u_gapX: options.gapX,
        u_gapY: options.gapY,
        u_strokeWidth: options.strokeWidth,
        u_sizeRange: options.sizeRange,
        u_opacityRange: options.opacityRange,
        u_shape: DOT_GRID_SHAPES[options.shape],
        ...sizingUniforms(options),
    };
}
