import { type JSX, omit } from "@destack/view";
import { Shader, type ShaderProperties } from "../shader/shader.tsx";
import { SIZING_KEYS } from "../shader/sizing.ts";
import { WATER_FRAGMENT, WATER_PROPERTIES, waterUniforms } from "./effect.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The properties of the water besides its declared ones: the shader's and its image. */
export type WaterAttributes = Omit<
    ShaderProperties,
    "fragmentShader" | "uniforms" | "speed" | "frame"
> & {
    /** The image it draws over, none for its background alone. */
    readonly image?: string | HTMLImageElement | null;
};

/** Draw the water. */
export const Water = defineComponent({
    name: "water",
    properties: WATER_PROPERTIES,
    render(properties: RenderProperties<WaterAttributes, typeof WATER_PROPERTIES>): JSX.Element {
        // pass what the shader takes besides the effect's properties
        const rest = omit(
            properties,
            ...SIZING_KEYS,
            "image",
            "colorBack",
            "colorHighlight",
            "highlights",
            "layering",
            "edges",
            "waves",
            "caustic",
            "size",
        );

        return (
            <Shader
                fragmentShader={WATER_FRAGMENT}
                uniforms={waterUniforms(properties)}
                {...rest}
            />
        );
    },
});

/** The properties of the water, the shader's included. */
export type WaterProperties = ComponentProperties<typeof Water>;
