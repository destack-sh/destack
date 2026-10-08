import { type JSX, merge, omit } from "@destack/view";
import { Shader } from "../shader/shader.tsx";
import { SIZING_KEYS } from "../shader/sizing.ts";
import {
    SIMPLEX_NOISE_DEFAULTS,
    SIMPLEX_NOISE_FRAGMENT,
    type SimplexNoiseProperties,
    simplexNoiseUniforms,
} from "./effect.ts";

/** Draw the simplex noise. */
export function SimplexNoise(properties: SimplexNoiseProperties): JSX.Element {
    // fill unset options from the defaults and pass the rest to the shader
    const options = merge(SIMPLEX_NOISE_DEFAULTS, properties);
    const rest = omit(options, ...SIZING_KEYS, "colors", "stepsPerColor", "softness");

    return (
        <Shader
            fragment={SIMPLEX_NOISE_FRAGMENT}
            uniforms={simplexNoiseUniforms(options)}
            {...rest}
        />
    );
}
