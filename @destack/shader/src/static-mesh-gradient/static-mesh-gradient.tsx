import { type JSX, omit } from "@destack/view";
import { Shader, type ShaderProperties } from "../shader/shader.tsx";
import { SIZING_KEYS } from "../shader/sizing.ts";
import {
    STATIC_MESH_GRADIENT_FRAGMENT,
    STATIC_MESH_GRADIENT_PROPERTIES,
    staticMeshGradientUniforms,
} from "./effect.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The properties of the static mesh gradient besides its declared ones: the shader's own. */
export type StaticMeshGradientAttributes = Omit<
    ShaderProperties,
    "fragmentShader" | "uniforms" | "speed" | "frame"
>;

/** Draw the static mesh gradient. */
export const StaticMeshGradient = defineComponent({
    name: "staticMeshGradient",
    properties: STATIC_MESH_GRADIENT_PROPERTIES,
    render(
        properties: RenderProperties<
            StaticMeshGradientAttributes,
            typeof STATIC_MESH_GRADIENT_PROPERTIES
        >,
    ): JSX.Element {
        // pass what the shader takes besides the effect's properties
        const rest = omit(
            properties,
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
                uniforms={staticMeshGradientUniforms(properties)}
                {...rest}
            />
        );
    },
});

/** The properties of the static mesh gradient, the shader's included. */
export type StaticMeshGradientProperties = ComponentProperties<typeof StaticMeshGradient>;
