import { type JSX, merge, omit } from "@destack/view";
import { Shader } from "../shader/shader.tsx";
import { SIZING_KEYS } from "../shader/sizing.ts";
import { WARP_DEFAULTS, WARP_FRAGMENT, type WarpProperties, warpUniforms } from "./effect.ts";

/** Draw the warp. */
export function Warp(properties: WarpProperties): JSX.Element {
    // fill unset options from the defaults and pass the rest to the shader
    const options = merge(WARP_DEFAULTS, properties);
    const rest = omit(
        options,
        ...SIZING_KEYS,
        "colors",
        "proportion",
        "softness",
        "distortion",
        "swirl",
        "swirlIterations",
        "shapeScale",
        "shape",
    );

    return <Shader fragment={WARP_FRAGMENT} uniforms={warpUniforms(options)} {...rest} />;
}
