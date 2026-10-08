import { type JSX, merge, omit } from "@destack/view";
import { Shader } from "../shader/shader.tsx";
import { SIZING_KEYS } from "../shader/sizing.ts";
import {
    FLUTED_GLASS_DEFAULTS,
    FLUTED_GLASS_FRAGMENT,
    type FlutedGlassProperties,
    flutedGlassUniforms,
} from "./effect.ts";

/** Draw the fluted glass. */
export function FlutedGlass(properties: FlutedGlassProperties): JSX.Element {
    // fill unset options from the defaults and pass the rest to the shader
    const options = merge(FLUTED_GLASS_DEFAULTS, properties);
    const rest = omit(
        options,
        ...SIZING_KEYS,
        "image",
        "colorBack",
        "colorShadow",
        "colorHighlight",
        "shadows",
        "size",
        "angle",
        "distortionShape",
        "highlights",
        "shape",
        "distortion",
        "shift",
        "blur",
        "edges",
        "stretch",
        "margin",
        "marginLeft",
        "marginRight",
        "marginTop",
        "marginBottom",
        "grainMixer",
        "grainOverlay",
    );

    return (
        <Shader
            fragmentShader={FLUTED_GLASS_FRAGMENT}
            uniforms={flutedGlassUniforms(options)}
            {...rest}
        />
    );
}
