import { type JSX, omit } from "@destack/view";
import { Shader, type ShaderProperties } from "../shader/shader.tsx";
import { SIZING_KEYS } from "../shader/sizing.ts";
import {
    PAPER_TEXTURE_FRAGMENT,
    PAPER_TEXTURE_PROPERTIES,
    paperTextureUniforms,
} from "./effect.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The properties of the paper texture besides its declared ones: the shader's and its image. */
export type PaperTextureAttributes = Omit<
    ShaderProperties,
    "fragmentShader" | "uniforms" | "speed" | "frame"
> & {
    /** The image it draws over, none for its background alone. */
    readonly image?: string | HTMLImageElement | null;
};

/** Draw the paper texture. */
export const PaperTexture = defineComponent({
    name: "paperTexture",
    properties: PAPER_TEXTURE_PROPERTIES,
    render(
        properties: RenderProperties<PaperTextureAttributes, typeof PAPER_TEXTURE_PROPERTIES>,
    ): JSX.Element {
        // pass what the shader takes besides the effect's properties
        const rest = omit(
            properties,
            ...SIZING_KEYS,
            "image",
            "colorBack",
            "colorPaper",
            "colorShadow",
            "blending",
            "distortion",
            "clip",
            "angle",
            "seed",
            "roughness",
            "roughnessSize",
            "roughnessRows",
            "fiber",
            "fiberSize",
            "folds",
            "foldSizeX",
            "foldSizeY",
            "foldOffsetX",
            "foldOffsetY",
            "wrinkles",
            "wrinkleSize",
            "crumples",
            "crumpleCount",
            "drops",
        );

        return (
            <Shader
                fragmentShader={PAPER_TEXTURE_FRAGMENT}
                uniforms={paperTextureUniforms(properties)}
                {...rest}
            />
        );
    },
});

/** The properties of the paper texture, the shader's included. */
export type PaperTextureProperties = ComponentProperties<typeof PaperTexture>;
