import { type JSX, merge, omit } from "@destack/view";
import { Shader } from "../shader/shader.tsx";
import { SIZING_KEYS } from "../shader/sizing.ts";
import {
    PULSING_BORDER_DEFAULTS,
    PULSING_BORDER_FRAGMENT,
    type PulsingBorderProperties,
    pulsingBorderUniforms,
} from "./effect.ts";

/** Draw the pulsing border. */
export function PulsingBorder(properties: PulsingBorderProperties): JSX.Element {
    // fill unset options from the defaults and pass the rest to the shader
    const options = merge(PULSING_BORDER_DEFAULTS, properties);
    const rest = omit(
        options,
        ...SIZING_KEYS,
        "colorBack",
        "colors",
        "roundness",
        "thickness",
        "margin",
        "marginLeft",
        "marginRight",
        "marginTop",
        "marginBottom",
        "aspectRatio",
        "softness",
        "intensity",
        "bloom",
        "spots",
        "spotSize",
        "pulse",
        "smoke",
        "smokeSize",
    );

    return (
        <Shader
            fragment={PULSING_BORDER_FRAGMENT}
            uniforms={pulsingBorderUniforms(options)}
            {...rest}
        />
    );
}
