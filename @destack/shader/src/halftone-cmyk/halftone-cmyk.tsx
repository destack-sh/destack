import { type JSX, merge, omit } from "@destack/view";
import { Shader } from "../shader/shader.tsx";
import { SIZING_KEYS } from "../shader/sizing.ts";
import {
    HALFTONE_CMYK_DEFAULTS,
    HALFTONE_CMYK_FRAGMENT,
    type HalftoneCmykProperties,
    halftoneCmykUniforms,
} from "./effect.ts";

/** Draw the halftone cmyk. */
export function HalftoneCmyk(properties: HalftoneCmykProperties): JSX.Element {
    // fill unset options from the defaults and pass the rest to the shader
    const options = merge(HALFTONE_CMYK_DEFAULTS, properties);
    const rest = omit(
        options,
        ...SIZING_KEYS,
        "image",
        "colorBack",
        "colorC",
        "colorM",
        "colorY",
        "colorK",
        "size",
        "contrast",
        "softness",
        "grainSize",
        "grainMixer",
        "grainOverlay",
        "gridNoise",
        "floodC",
        "floodM",
        "floodY",
        "floodK",
        "gainC",
        "gainM",
        "gainY",
        "gainK",
        "type",
    );

    return (
        <Shader
            fragment={HALFTONE_CMYK_FRAGMENT}
            uniforms={halftoneCmykUniforms(options)}
            {...rest}
        />
    );
}
