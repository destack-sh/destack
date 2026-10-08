import { type JSX, merge, omit } from "@destack/view";
import { Shader } from "../shader/shader.tsx";
import { SIZING_KEYS } from "../shader/sizing.ts";
import {
    DITHERING_DEFAULTS,
    DITHERING_FRAGMENT,
    type DitheringProperties,
    ditheringUniforms,
} from "./effect.ts";

/** Draw the dithering. */
export function Dithering(properties: DitheringProperties): JSX.Element {
    // fill unset options from the defaults and pass the rest to the shader
    const options = merge(DITHERING_DEFAULTS, properties);
    const rest = omit(options, ...SIZING_KEYS, "colorBack", "colorFront", "shape", "type", "size");

    return (
        <Shader
            fragmentShader={DITHERING_FRAGMENT}
            uniforms={ditheringUniforms(options)}
            {...rest}
        />
    );
}
