import { type JSX, omit } from "@destack/view";
import { Shader, type ShaderProperties } from "../shader/shader.tsx";
import { SIZING_KEYS } from "../shader/sizing.ts";
import { DOT_GRID_FRAGMENT, DOT_GRID_PROPERTIES, dotGridUniforms } from "./effect.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The properties of the dot grid besides its declared ones: the shader's own. */
export type DotGridAttributes = Omit<
    ShaderProperties,
    "fragmentShader" | "uniforms" | "speed" | "frame"
>;

/** Draw the dot grid. */
export const DotGrid = defineComponent({
    name: "dotGrid",
    properties: DOT_GRID_PROPERTIES,
    render(
        properties: RenderProperties<DotGridAttributes, typeof DOT_GRID_PROPERTIES>,
    ): JSX.Element {
        // pass what the shader takes besides the effect's properties
        const rest = omit(
            properties,
            ...SIZING_KEYS,
            "colorBack",
            "colorFill",
            "colorStroke",
            "size",
            "gapX",
            "gapY",
            "strokeWidth",
            "sizeRange",
            "opacityRange",
            "shape",
        );

        return (
            <Shader
                fragmentShader={DOT_GRID_FRAGMENT}
                uniforms={dotGridUniforms(properties)}
                {...rest}
            />
        );
    },
});

/** The properties of the dot grid, the shader's included. */
export type DotGridProperties = ComponentProperties<typeof DotGrid>;
