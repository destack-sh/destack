import { type JSX, omit } from "@destack/view";
import { Shader, type ShaderProperties } from "../shader/shader.tsx";
import { SIZING_KEYS } from "../shader/sizing.ts";
import { WARP_FRAGMENT, WARP_PROPERTIES, warpUniforms } from "./effect.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The properties of the warp besides its declared ones: the shader's own. */
export type WarpAttributes = Omit<
    ShaderProperties,
    "fragmentShader" | "uniforms" | "speed" | "frame"
>;

/** Draw the warp. */
export const Warp = defineComponent({
    name: "warp",
    properties: WARP_PROPERTIES,
    render(properties: RenderProperties<WarpAttributes, typeof WARP_PROPERTIES>): JSX.Element {
        // pass what the shader takes besides the effect's properties
        const rest = omit(
            properties,
            ...SIZING_KEYS,
            "colors",
            "proportion",
            "softness",
            "distortion",
            "swirl",
            "swirlIterations",
            "shapeScale",
            "shape",
        );

        return (
            <Shader fragmentShader={WARP_FRAGMENT} uniforms={warpUniforms(properties)} {...rest} />
        );
    },
});

/** The properties of the warp, the shader's included. */
export type WarpProperties = ComponentProperties<typeof Warp>;
