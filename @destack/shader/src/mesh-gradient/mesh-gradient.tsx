import { type JSX, merge, omit } from "@destack/view";
import { Shader } from "../shader/shader.tsx";
import { SIZING_KEYS } from "../shader/sizing.ts";
import {
    MESH_GRADIENT_DEFAULTS,
    MESH_GRADIENT_FRAGMENT,
    type MeshGradientProperties,
    meshGradientUniforms,
} from "./effect.ts";

/** Draw the mesh gradient. */
export function MeshGradient(properties: MeshGradientProperties): JSX.Element {
    // fill unset options from the defaults and pass the rest to the shader
    const options = merge(MESH_GRADIENT_DEFAULTS, properties);
    const rest = omit(
        options,
        ...SIZING_KEYS,
        "colors",
        "distortion",
        "swirl",
        "grainMixer",
        "grainOverlay",
    );

    return (
        <Shader
            fragment={MESH_GRADIENT_FRAGMENT}
            uniforms={meshGradientUniforms(options)}
            {...rest}
        />
    );
}
