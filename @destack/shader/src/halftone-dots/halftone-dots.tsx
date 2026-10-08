import { type JSX, merge, omit } from "@destack/view";
import { Shader } from "../shader/shader.tsx";
import { SIZING_KEYS } from "../shader/sizing.ts";
import {
    HALFTONE_DOTS_DEFAULTS,
    HALFTONE_DOTS_FRAGMENT,
    type HalftoneDotsProperties,
    halftoneDotsUniforms,
} from "./effect.ts";

/** Draw the halftone dots. */
export function HalftoneDots(properties: HalftoneDotsProperties): JSX.Element {
    // fill unset options from the defaults and pass the rest to the shader
    const options = merge(HALFTONE_DOTS_DEFAULTS, properties);
    const rest = omit(
        options,
        ...SIZING_KEYS,
        "image",
        "colorBack",
        "colorFront",
        "size",
        "radius",
        "contrast",
        "originalColors",
        "inverted",
        "grainMixer",
        "grainOverlay",
        "grainSize",
        "grid",
        "type",
    );

    return (
        <Shader
            fragment={HALFTONE_DOTS_FRAGMENT}
            uniforms={halftoneDotsUniforms(options)}
            {...rest}
        />
    );
}
