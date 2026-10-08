import { glsl } from "../glsl/glsl.ts";
import { NOISE_TEXTURE } from "../shader/noise.ts";
import { OBJECT_SIZING, type Sizing, sizingUniforms } from "../shader/sizing.ts";
import { type ShaderColor, type ShaderValues } from "../shader/value.ts";
import type { ShaderProperties } from "../shader/shader.tsx";

/** The most colors in the pulsing border. */
export const PULSING_BORDER_MAX_COLOR_COUNT = 5;

/** The most spots in the pulsing border. */
export const PULSING_BORDER_MAX_SPOTS = 4;

/** A aspectratio of the pulsing border. */
export type PulsingBorderAspectRatio = "auto" | "square";

/** The number the fragment shader reads for each aspectratio. */
export const PULSING_BORDER_ASPECT_RATIOS: Readonly<Record<PulsingBorderAspectRatio, number>> = {
    auto: 0,
    square: 1,
};

/** The fragment shader of the pulsing border. */
export const PULSING_BORDER_FRAGMENT = `#version 300 es
precision lowp float;

uniform float u_time;

uniform vec4 u_colorBack;
uniform vec4 u_colors[${PULSING_BORDER_MAX_COLOR_COUNT}];
uniform float u_colorsCount;
uniform float u_roundness;
uniform float u_thickness;
uniform float u_marginLeft;
uniform float u_marginRight;
uniform float u_marginTop;
uniform float u_marginBottom;
uniform float u_aspectRatio;
uniform float u_softness;
uniform float u_intensity;
uniform float u_bloom;
uniform float u_spotSize;
uniform float u_spots;
uniform float u_pulse;
uniform float u_smoke;
uniform float u_smokeSize;

uniform sampler2D u_noiseTexture;

in vec2 v_responsiveUV;
in vec2 v_responsiveBoxGivenSize;
in vec2 v_patternUV;

out vec4 fragColor;

${glsl.pi}

float beat(float time) {
  float first = pow(abs(sin(time * TWO_PI)), 10.);
  float second = pow(abs(sin((time - .15) * TWO_PI)), 10.);

  return clamp(first + 0.6 * second, 0.0, 1.0);
}

float sst(float edge0, float edge1, float x) {
  return smoothstep(edge0, edge1, x);
}

float roundedBox(vec2 uv, vec2 halfSize, float distance, float cornerDistance, float thickness, float softness) {
  float borderDistance = abs(distance);
  float aa = 2. * fwidth(distance);
  float border = 1. - sst(min(mix(thickness, -thickness, softness), thickness + aa), max(mix(thickness, -thickness, softness), thickness + aa), borderDistance);
  float cornerFadeCircles = 0.;
  cornerFadeCircles = mix(1., cornerFadeCircles, sst(0., 1., length((uv + halfSize) / thickness)));
  cornerFadeCircles = mix(1., cornerFadeCircles, sst(0., 1., length((uv - vec2(-halfSize.x, halfSize.y)) / thickness)));
  cornerFadeCircles = mix(1., cornerFadeCircles, sst(0., 1., length((uv - vec2(halfSize.x, -halfSize.y)) / thickness)));
  cornerFadeCircles = mix(1., cornerFadeCircles, sst(0., 1., length((uv - halfSize) / thickness)));
  aa = fwidth(cornerDistance);
  float cornerFade = sst(0., mix(aa, thickness, softness), cornerDistance);
  cornerFade *= cornerFadeCircles;
  border += cornerFade;
  return border;
}

${glsl.randomGB}

float randomG(vec2 p) {
  vec2 uv = floor(p) / 100. + .5;
  return texture(u_noiseTexture, fract(uv)).g;
}
float valueNoise(vec2 st) {
  vec2 i = floor(st);
  vec2 f = fract(st);
  float a = randomG(i);
  float b = randomG(i + vec2(1.0, 0.0));
  float c = randomG(i + vec2(0.0, 1.0));
  float d = randomG(i + vec2(1.0, 1.0));
  vec2 u = f * f * (3.0 - 2.0 * f);
  float x1 = mix(a, b, u.x);
  float x2 = mix(c, d, u.x);
  return mix(x1, x2, u.y);
}

void main() {
  const float firstFrameOffset = 109.;
  float t = 1.2 * (u_time + firstFrameOffset);

  vec2 borderUV = v_responsiveUV;
  float pulse = u_pulse * beat(.18 * u_time);

  float canvasRatio = v_responsiveBoxGivenSize.x / v_responsiveBoxGivenSize.y;
  vec2 halfSize = vec2(.5);
  borderUV.x *= max(canvasRatio, 1.);
  borderUV.y /= min(canvasRatio, 1.);
  halfSize.x *= max(canvasRatio, 1.);
  halfSize.y /= min(canvasRatio, 1.);

  float mL = u_marginLeft;
  float mR = u_marginRight;
  float mT = u_marginTop;
  float mB = u_marginBottom;
  float mX = mL + mR;
  float mY = mT + mB;

  if (u_aspectRatio > 0.) {
    float shapeRatio = canvasRatio * (1. - mX) / max(1. - mY, 1e-6);
    float freeX = shapeRatio > 1. ? (1. - mX) * (1. - 1. / max(abs(shapeRatio), 1e-6)) : 0.;
    float freeY = shapeRatio < 1. ? (1. - mY) * (1. - shapeRatio) : 0.;
    mL += freeX * 0.5;
    mR += freeX * 0.5;
    mT += freeY * 0.5;
    mB += freeY * 0.5;
    mX = mL + mR;
    mY = mT + mB;
  }

  float thickness = .5 * u_thickness * min(halfSize.x, halfSize.y);

  halfSize.x *= (1. - mX);
  halfSize.y *= (1. - mY);

  vec2 centerShift = vec2(
  (mL - mR) * max(canvasRatio, 1.) * 0.5,
  (mB - mT) / min(canvasRatio, 1.) * 0.5
  );

  borderUV -= centerShift;
  halfSize -= mix(thickness, 0., u_softness);

  float radius = mix(0., min(halfSize.x, halfSize.y), u_roundness);
  vec2 d = abs(borderUV) - halfSize + radius;
  float outsideDistance = length(max(d, .0001)) - radius;
  float insideDistance = min(max(d.x, d.y), .0001);
  float cornerDistance = abs(min(max(d.x, d.y) - .45 * radius, .0));
  float distance = outsideDistance + insideDistance;

  float borderThickness = mix(thickness, 3. * thickness, u_softness);
  float border = roundedBox(borderUV, halfSize, distance, cornerDistance, borderThickness, u_softness);
  border = pow(border, 1. + u_softness);

  vec2 smokeUV = .3 * u_smokeSize * v_patternUV;
  float smoke = clamp(3. * valueNoise(2.7 * smokeUV + .5 * t), 0., 1.);
  smoke -= valueNoise(3.4 * smokeUV - .5 * t);
  float smokeThickness = thickness + .2;
  smokeThickness = min(.4, max(smokeThickness, .1));
  smoke *= roundedBox(borderUV, halfSize, distance, cornerDistance, smokeThickness, 1.);
  smoke = 30. * smoke * smoke;
  smoke *= mix(0., .5, pow(u_smoke, 2.));
  smoke *= mix(1., pulse, u_pulse);
  smoke = clamp(smoke, 0., 1.);
  border += smoke;

  border = clamp(border, 0., 1.);

  vec3 blendColor = vec3(0.);
  float blendAlpha = 0.;
  vec3 addColor = vec3(0.);
  float addAlpha = 0.;

  float bloom = 4. * u_bloom;
  float intensity = 1. + (1. + 4. * u_softness) * u_intensity;

  float angle = atan(borderUV.y, borderUV.x) / TWO_PI;

  for (int colorIdx = 0; colorIdx < ${PULSING_BORDER_MAX_COLOR_COUNT}; colorIdx++) {
    if (colorIdx >= int(u_colorsCount)) break;
    float colorIdxF = float(colorIdx);

    vec3 c = u_colors[colorIdx].rgb * u_colors[colorIdx].a;
    float a = u_colors[colorIdx].a;

    for (int spotIdx = 0; spotIdx < ${PULSING_BORDER_MAX_SPOTS}; spotIdx++) {
      if (spotIdx >= int(u_spots)) break;
      float spotIdxF = float(spotIdx);

      vec2 randVal = randomGB(vec2(spotIdxF * 10. + 2., 40. + colorIdxF));

      float time = (.1 + .15 * abs(sin(spotIdxF * (2. + colorIdxF)) * cos(spotIdxF * (2. + 2.5 * colorIdxF)))) * t + randVal.x * 3.;
      time *= mix(1., -1., step(.5, randVal.y));

      float mask = .5 + .5 * mix(
      sin(t + spotIdxF * (5. - 1.5 * colorIdxF)),
      cos(t + spotIdxF * (3. + 1.3 * colorIdxF)),
      step(mod(colorIdxF, 2.), .5)
      );

      float p = clamp(2. * u_pulse - randVal.x, 0., 1.);
      mask = mix(mask, pulse, p);

      float atg1 = fract(angle + time);
      float spotSize = .05 + .6 * pow(u_spotSize, 2.) + .05 * randVal.x;
      spotSize = mix(spotSize, .1, p);
      float sector = sst(.5 - spotSize, .5, atg1) * (1. - sst(.5, .5 + spotSize, atg1));

      sector *= mask;
      sector *= border;
      sector *= intensity;
      sector = clamp(sector, 0., 1.);

      vec3 srcColor = c * sector;
      float srcAlpha = a * sector;

      blendColor += ((1. - blendAlpha) * srcColor);
      blendAlpha = blendAlpha + (1. - blendAlpha) * srcAlpha;
      addColor += srcColor;
      addAlpha += srcAlpha;
    }
  }

  vec3 accumColor = mix(blendColor, addColor, bloom);
  float accumAlpha = mix(blendAlpha, addAlpha, bloom);
  accumAlpha = clamp(accumAlpha, 0., 1.);

  vec3 bgColor = u_colorBack.rgb * u_colorBack.a;
  vec3 color = accumColor + (1. - accumAlpha) * bgColor;
  float opacity = accumAlpha + (1. - accumAlpha) * u_colorBack.a;

  ${glsl.dither}

  fragColor = vec4(color, opacity);
}`;

/** The options of the pulsing border, beside its sizing. */
export interface PulsingBorderOptions extends Sizing, Pick<ShaderProperties, "speed" | "frame"> {
    /** The background color. */
    readonly colorBack?: ShaderColor;
    /** Up to 5 spot colors. */
    readonly colors?: readonly ShaderColor[];
    /** The border radius, from 0 to 1. */
    readonly roundness?: number;
    /** The border base width, from 0 to 1. */
    readonly thickness?: number;
    /** The margin. */
    readonly margin?: number;
    /** The distance from the left edge to the effect, from 0 to 1. */
    readonly marginLeft?: number;
    /** The distance from the right edge to the effect, from 0 to 1. */
    readonly marginRight?: number;
    /** The distance from the top edge to the effect, from 0 to 1. */
    readonly marginTop?: number;
    /** The distance from the bottom edge to the effect, from 0 to 1. */
    readonly marginBottom?: number;
    /** The aspect ratio mode. */
    readonly aspectRatio?: PulsingBorderAspectRatio;
    /** The border edge sharpness, 0 = hard edge, 1 = smooth gradient, from 0 to 1. */
    readonly softness?: number;
    /** The thickness of individual color spots, from 0 to 1. */
    readonly intensity?: number;
    /** The power of glow, 0 = normal blending, 1 = additive blending, from 0 to 1. */
    readonly bloom?: number;
    /** The number of spots added for each color, from 1 to 4. */
    readonly spots?: number;
    /** The angular size of spots, from 0 to 1. */
    readonly spotSize?: number;
    /** The optional pulsing animation intensity, from 0 to 1. */
    readonly pulse?: number;
    /** The optional noisy shape extending the border, from 0 to 1. */
    readonly smoke?: number;
    /** The size of the smoke effect, needs smoke > 0, from 0 to 1. */
    readonly smokeSize?: number;
}

/** The properties of the pulsing border, the shader's included. */
export interface PulsingBorderProperties
    extends PulsingBorderOptions, Omit<ShaderProperties, "fragmentShader" | "uniforms"> {}

/** The default pulsing border. */
export const PULSING_BORDER_DEFAULTS: Required<PulsingBorderOptions> = {
    ...OBJECT_SIZING,
    speed: 1,
    frame: 0,
    scale: 0.6,
    colorBack: "#000000",
    colors: ["#0dc1fd", "#d915ef", "#ff3f2ecc"],
    roundness: 0.25,
    thickness: 0.1,
    margin: 0,
    marginLeft: 0,
    marginRight: 0,
    marginTop: 0,
    marginBottom: 0,
    aspectRatio: "auto",
    softness: 0.75,
    intensity: 0.2,
    bloom: 0.25,
    spots: 4,
    spotSize: 0.5,
    pulse: 0.25,
    smoke: 0.3,
    smokeSize: 0.6,
};

/** Write pulsing border options as the fragment shader's uniforms. */
export function pulsingBorderUniforms(options: Required<PulsingBorderOptions>): ShaderValues {
    return {
        u_colorBack: options.colorBack,
        u_colors: options.colors,
        u_colorsCount: options.colors.length,
        u_roundness: options.roundness,
        u_thickness: options.thickness,
        u_marginLeft: options.marginLeft,
        u_marginRight: options.marginRight,
        u_marginTop: options.marginTop,
        u_marginBottom: options.marginBottom,
        u_aspectRatio: PULSING_BORDER_ASPECT_RATIOS[options.aspectRatio],
        u_softness: options.softness,
        u_intensity: options.intensity,
        u_bloom: options.bloom,
        u_spots: options.spots,
        u_spotSize: options.spotSize,
        u_pulse: options.pulse,
        u_smoke: options.smoke,
        u_smokeSize: options.smokeSize,
        u_noiseTexture: NOISE_TEXTURE,
        ...sizingUniforms(options),
    };
}
