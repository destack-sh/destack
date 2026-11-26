import { type JSX, omit } from "@destack/view";
import { Shader, type ShaderProperties } from "../shader/shader.tsx";
import { SIZING_KEYS } from "../shader/sizing.ts";
import {
    HALFTONE_DOTS_FRAGMENT,
    HALFTONE_DOTS_PROPERTIES,
    halftoneDotsUniforms,
} from "./effect.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The properties of the halftone dots besides its declared ones: the shader's and its image. */
export type HalftoneDotsAttributes = Omit<
    ShaderProperties,
    "fragmentShader" | "uniforms" | "speed" | "frame"
> & {
    /** The image it draws over, none for its background alone. */
    readonly image?: string | HTMLImageElement | null;
};

/** Draw the halftone dots. */
export const HalftoneDots = defineComponent({
    name: "halftoneDots",
    properties: HALFTONE_DOTS_PROPERTIES,
    render(
        properties: RenderProperties<HalftoneDotsAttributes, typeof HALFTONE_DOTS_PROPERTIES>,
    ): JSX.Element {
        // pass what the shader takes besides the effect's properties
        const rest = omit(
            properties,
            ...SIZING_KEYS,
            "image",
            "colorBack",
            "colorFront",
            "size",
            "radius",
            "contrast",
            "originalColors",
            "inverted",
            "grainMixer",
            "grainOverlay",
            "grainSize",
            "grid",
            "type",
        );

        return (
            <Shader
                fragmentShader={HALFTONE_DOTS_FRAGMENT}
                uniforms={halftoneDotsUniforms(properties)}
                {...rest}
            />
        );
    },
});

/** The properties of the halftone dots, the shader's included. */
export type HalftoneDotsProperties = ComponentProperties<typeof HalftoneDots>;
