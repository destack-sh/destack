import { type JSX, merge, omit } from "@destack/view";
import { Shader } from "../shader/shader.tsx";
import { SIZING_KEYS } from "../shader/sizing.ts";
import { WAVES_DEFAULTS, WAVES_FRAGMENT, type WavesProperties, wavesUniforms } from "./effect.ts";

/** Draw the waves. */
export function Waves(properties: WavesProperties): JSX.Element {
    // fill unset options from the defaults and pass the rest to the shader
    const options = merge(WAVES_DEFAULTS, properties);
    const rest = omit(
        options,
        ...SIZING_KEYS,
        "colorFront",
        "colorBack",
        "shape",
        "frequency",
        "amplitude",
        "spacing",
        "proportion",
        "softness",
    );

    return <Shader fragment={WAVES_FRAGMENT} uniforms={wavesUniforms(options)} {...rest} />;
}
