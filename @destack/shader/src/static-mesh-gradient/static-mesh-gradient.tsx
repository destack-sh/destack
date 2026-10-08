import { type JSX, merge, omit } from "@destack/view";
import { Shader } from "../shader/shader.tsx";
import { SIZING_KEYS } from "../shader/sizing.ts";
import {
    STATIC_MESH_GRADIENT_DEFAULTS,
    STATIC_MESH_GRADIENT_FRAGMENT,
    type StaticMeshGradientProperties,
    staticMeshGradientUniforms,
} from "./effect.ts";

/** Draw the static mesh gradient. */
export function StaticMeshGradient(properties: StaticMeshGradientProperties): JSX.Element {
    // fill unset options from the defaults and pass the rest to the shader
    const options = merge(STATIC_MESH_GRADIENT_DEFAULTS, properties);
    const rest = omit(
        options,
        ...SIZING_KEYS,
        "colors",
        "positions",
        "waveX",
        "waveXShift",
        "waveY",
        "waveYShift",
        "mixing",
        "grainMixer",
        "grainOverlay",
    );

    return (
        <Shader
            fragmentShader={STATIC_MESH_GRADIENT_FRAGMENT}
            uniforms={staticMeshGradientUniforms(options)}
            {...rest}
        />
    );
}
