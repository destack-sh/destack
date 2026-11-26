import { type JSX, omit } from "@destack/view";
import { Shader, type ShaderProperties } from "../shader/shader.tsx";
import { SIZING_KEYS } from "../shader/sizing.ts";
import {
    IMAGE_DITHERING_FRAGMENT,
    IMAGE_DITHERING_PROPERTIES,
    imageDitheringUniforms,
} from "./effect.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The properties of the image dithering besides its declared ones: the shader's and its image. */
export type ImageDitheringAttributes = Omit<
    ShaderProperties,
    "fragmentShader" | "uniforms" | "speed" | "frame"
> & {
    /** The image it draws over, none for its background alone. */
    readonly image?: string | HTMLImageElement | null;
};

/** Draw the image dithering. */
export const ImageDithering = defineComponent({
    name: "imageDithering",
    properties: IMAGE_DITHERING_PROPERTIES,
    render(
        properties: RenderProperties<ImageDitheringAttributes, typeof IMAGE_DITHERING_PROPERTIES>,
    ): JSX.Element {
        // pass what the shader takes besides the effect's properties
        const rest = omit(
            properties,
            ...SIZING_KEYS,
            "image",
            "colorFront",
            "colorBack",
            "colorHighlight",
            "type",
            "size",
            "colorSteps",
            "originalColors",
            "inverted",
        );

        return (
            <Shader
                fragmentShader={IMAGE_DITHERING_FRAGMENT}
                uniforms={imageDitheringUniforms(properties)}
                {...rest}
            />
        );
    },
});

/** The properties of the image dithering, the shader's included. */
export type ImageDitheringProperties = ComponentProperties<typeof ImageDithering>;
