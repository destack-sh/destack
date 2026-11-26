import { type JSX, omit } from "@destack/view";
import { Shader, type ShaderProperties } from "../shader/shader.tsx";
import { SIZING_KEYS } from "../shader/sizing.ts";
import {
    MESH_GRADIENT_FRAGMENT,
    MESH_GRADIENT_PROPERTIES,
    meshGradientUniforms,
} from "./effect.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The properties of the mesh gradient besides its declared ones: the shader's own. */
export type MeshGradientAttributes = Omit<
    ShaderProperties,
    "fragmentShader" | "uniforms" | "speed" | "frame"
>;

/** Draw the mesh gradient. */
export const MeshGradient = defineComponent({
    name: "meshGradient",
    properties: MESH_GRADIENT_PROPERTIES,
    render(
        properties: RenderProperties<MeshGradientAttributes, typeof MESH_GRADIENT_PROPERTIES>,
    ): JSX.Element {
        // pass what the shader takes besides the effect's properties
        const rest = omit(
            properties,
            ...SIZING_KEYS,
            "colors",
            "distortion",
            "swirl",
            "grainMixer",
            "grainOverlay",
        );

        return (
            <Shader
                fragmentShader={MESH_GRADIENT_FRAGMENT}
                uniforms={meshGradientUniforms(properties)}
                {...rest}
            />
        );
    },
});

/** The properties of the mesh gradient, the shader's included. */
export type MeshGradientProperties = ComponentProperties<typeof MeshGradient>;
