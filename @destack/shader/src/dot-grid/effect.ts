import { Color } from "@destack/style";
import { t } from "@destack/locale";
import { field } from "@destack/object";
import { schema } from "@destack/schema";
import type { DeclaredProperties } from "@destack/view/declare";
import { Shader } from "../shader/shader.tsx";
import { PATTERN_SIZING_PROPERTIES, sizingUniforms } from "../shader/sizing.ts";
import { COLOR_BACK } from "../shader/option.ts";
import { glsl } from "../glsl/glsl.ts";
import { type ShaderValues } from "../shader/value.ts";

/** The shapes of the dot grid, as a person picks one. */
const DOT_GRID_SHAPE_CHOICES = {
    circle: { title: t.context("option")`Circle` },
    diamond: { title: t.context("option")`Diamond` },
    square: { title: t.context("option")`Square` },
    triangle: { title: t.context("option")`Triangle` },
};

/** A shape of the dot grid. */
export type DotGridShape = keyof typeof DOT_GRID_SHAPE_CHOICES;

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

/** The properties people change in the dot grid's controls and inspector. */
export const DOT_GRID_PROPERTIES = {
    ...Shader.properties,
    ...PATTERN_SIZING_PROPERTIES,
    colorBack: COLOR_BACK.default("#000000"),
    colorFill: field
        .json(Color.schema)
        .labeled(t.context("label")`Fill`)
        .described(t`The color inside each shape.`)
        .default("#ffffff"),
    colorStroke: field
        .json(Color.schema)
        .labeled(t.context("label")`Stroke`)
        .described(t`The color of each shape's outline.`)
        .default("#ffaa00"),
    size: field
        .number(schema.number().min(0))
        .labeled(t.context("label")`Size`)
        .described(t`The size of each shape, in pixels.`)
        .default(2),
    gapX: field
        .number(schema.number().min(2).max(500))
        .labeled(t.context("label")`Horizontal gap`)
        .described(t`The space between columns, in pixels.`)
        .default(32),
    gapY: field
        .number(schema.number().min(2).max(500))
        .labeled(t.context("label")`Vertical gap`)
        .described(t`The space between rows, in pixels.`)
        .default(32),
    strokeWidth: field
        .number(schema.number().min(0).max(50))
        .labeled(t.context("label")`Stroke width`)
        .described(t`The width of each shape's outline, in pixels.`)
        .default(0),
    sizeRange: field
        .number(schema.number().min(0).max(1))
        .labeled(t.context("label")`Size variation`)
        .described(t`How much the size of each shape varies.`)
        .default(0),
    opacityRange: field
        .number(schema.number().min(0).max(1))
        .labeled(t.context("label")`Opacity variation`)
        .described(t`How much the opacity of each shape varies.`)
        .default(0),
    shape: field
        .enum(DOT_GRID_SHAPE_CHOICES)
        .labeled(t.context("label")`Shape`)
        .described(t`The shape of each dot.`)
        .default("circle"),
};

/** The options of the dot grid, each with its value. */
export type DotGridOptions = DeclaredProperties<typeof DOT_GRID_PROPERTIES>;

/** Write dot grid options as the fragment shader's uniforms. */
export function dotGridUniforms(options: DotGridOptions): ShaderValues {
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
