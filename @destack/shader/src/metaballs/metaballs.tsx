import { type JSX, omit } from "@destack/view";
import { Shader, type ShaderProperties } from "../shader/shader.tsx";
import { SIZING_KEYS } from "../shader/sizing.ts";
import { METABALLS_FRAGMENT, METABALLS_PROPERTIES, metaballsUniforms } from "./effect.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The properties of the metaballs besides its declared ones: the shader's own. */
export type MetaballsAttributes = Omit<
    ShaderProperties,
    "fragmentShader" | "uniforms" | "speed" | "frame"
>;

/** Draw the metaballs. */
export const Metaballs = defineComponent({
    name: "metaballs",
    properties: METABALLS_PROPERTIES,
    render(
        properties: RenderProperties<MetaballsAttributes, typeof METABALLS_PROPERTIES>,
    ): JSX.Element {
        // pass what the shader takes besides the effect's properties
        const rest = omit(properties, ...SIZING_KEYS, "colorBack", "colors", "count", "size");

        return (
            <Shader
                fragmentShader={METABALLS_FRAGMENT}
                uniforms={metaballsUniforms(properties)}
                {...rest}
            />
        );
    },
});

/** The properties of the metaballs, the shader's included. */
export type MetaballsProperties = ComponentProperties<typeof Metaballs>;
