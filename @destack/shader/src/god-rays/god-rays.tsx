import { type JSX, merge, omit } from "@destack/view";
import { Shader } from "../shader/shader.tsx";
import { SIZING_KEYS } from "../shader/sizing.ts";
import {
    GOD_RAYS_DEFAULTS,
    GOD_RAYS_FRAGMENT,
    type GodRaysProperties,
    godRaysUniforms,
} from "./effect.ts";

/** Draw the god rays. */
export function GodRays(properties: GodRaysProperties): JSX.Element {
    // fill unset options from the defaults and pass the rest to the shader
    const options = merge(GOD_RAYS_DEFAULTS, properties);
    const rest = omit(
        options,
        ...SIZING_KEYS,
        "colorBack",
        "colorBloom",
        "colors",
        "density",
        "spotty",
        "midIntensity",
        "midSize",
        "intensity",
        "bloom",
    );

    return (
        <Shader fragmentShader={GOD_RAYS_FRAGMENT} uniforms={godRaysUniforms(options)} {...rest} />
    );
}
