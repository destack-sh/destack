import { type JSX, merge, omit } from "@destack/view";
import { Shader } from "../shader/shader.tsx";
import { SIZING_KEYS } from "../shader/sizing.ts";
import {
    GRAIN_GRADIENT_DEFAULTS,
    GRAIN_GRADIENT_FRAGMENT,
    type GrainGradientProperties,
    grainGradientUniforms,
} from "./effect.ts";

/** Draw the grain gradient. */
export function GrainGradient(properties: GrainGradientProperties): JSX.Element {
    // fill unset options from the defaults and pass the rest to the shader
    const options = merge(GRAIN_GRADIENT_DEFAULTS, properties);
    const rest = omit(
        options,
        ...SIZING_KEYS,
        "colorBack",
        "colors",
        "softness",
        "intensity",
        "noise",
        "shape",
    );

    return (
        <Shader
            fragment={GRAIN_GRADIENT_FRAGMENT}
            uniforms={grainGradientUniforms(options)}
            {...rest}
        />
    );
}
