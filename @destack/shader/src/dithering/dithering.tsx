import { type JSX, omit } from "@destack/view";
import { Shader, type ShaderProperties } from "../shader/shader.tsx";
import { SIZING_KEYS } from "../shader/sizing.ts";
import { DITHERING_FRAGMENT, DITHERING_PROPERTIES, ditheringUniforms } from "./effect.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The properties of the dithering besides its declared ones: the shader's own. */
export type DitheringAttributes = Omit<
    ShaderProperties,
    "fragmentShader" | "uniforms" | "speed" | "frame"
>;

/** Draw the dithering. */
export const Dithering = defineComponent({
    name: "dithering",
    properties: DITHERING_PROPERTIES,
    render(
        properties: RenderProperties<DitheringAttributes, typeof DITHERING_PROPERTIES>,
    ): JSX.Element {
        // pass what the shader takes besides the effect's properties
        const rest = omit(
            properties,
            ...SIZING_KEYS,
            "colorBack",
            "colorFront",
            "shape",
            "type",
            "size",
        );

        return (
            <Shader
                fragmentShader={DITHERING_FRAGMENT}
                uniforms={ditheringUniforms(properties)}
                {...rest}
            />
        );
    },
});

/** The properties of the dithering, the shader's included. */
export type DitheringProperties = ComponentProperties<typeof Dithering>;
