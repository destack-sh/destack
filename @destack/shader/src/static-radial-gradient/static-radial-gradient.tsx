import { type JSX, merge, omit } from "@destack/view";
import { Shader } from "../shader/shader.tsx";
import { SIZING_KEYS } from "../shader/sizing.ts";
import {
    STATIC_RADIAL_GRADIENT_DEFAULTS,
    STATIC_RADIAL_GRADIENT_FRAGMENT,
    type StaticRadialGradientProperties,
    staticRadialGradientUniforms,
} from "./effect.ts";

/** Draw the static radial gradient. */
export function StaticRadialGradient(properties: StaticRadialGradientProperties): JSX.Element {
    // fill unset options from the defaults and pass the rest to the shader
    const options = merge(STATIC_RADIAL_GRADIENT_DEFAULTS, properties);
    const rest = omit(
        options,
        ...SIZING_KEYS,
        "colorBack",
        "colors",
        "radius",
        "focalDistance",
        "focalAngle",
        "falloff",
        "mixing",
        "distortion",
        "distortionShift",
        "distortionFreq",
        "grainMixer",
        "grainOverlay",
    );

    return (
        <Shader
            fragmentShader={STATIC_RADIAL_GRADIENT_FRAGMENT}
            uniforms={staticRadialGradientUniforms(options)}
            {...rest}
        />
    );
}
