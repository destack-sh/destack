import { type JSX, omit } from "@destack/view";
import { Shader, type ShaderProperties } from "../shader/shader.tsx";
import { SIZING_KEYS } from "../shader/sizing.ts";
import { GOD_RAYS_FRAGMENT, GOD_RAYS_PROPERTIES, godRaysUniforms } from "./effect.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The properties of the god rays besides its declared ones: the shader's own. */
export type GodRaysAttributes = Omit<
    ShaderProperties,
    "fragmentShader" | "uniforms" | "speed" | "frame"
>;

/** Draw the god rays. */
export const GodRays = defineComponent({
    name: "godRays",
    properties: GOD_RAYS_PROPERTIES,
    render(
        properties: RenderProperties<GodRaysAttributes, typeof GOD_RAYS_PROPERTIES>,
    ): JSX.Element {
        // pass what the shader takes besides the effect's properties
        const rest = omit(
            properties,
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
            <Shader
                fragmentShader={GOD_RAYS_FRAGMENT}
                uniforms={godRaysUniforms(properties)}
                {...rest}
            />
        );
    },
});

/** The properties of the god rays, the shader's included. */
export type GodRaysProperties = ComponentProperties<typeof GodRays>;
