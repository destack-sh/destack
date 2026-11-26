import { type JSX, omit } from "@destack/view";
import { Shader, type ShaderProperties } from "../shader/shader.tsx";
import { SIZING_KEYS } from "../shader/sizing.ts";
import {
    HALFTONE_CMYK_FRAGMENT,
    HALFTONE_CMYK_PROPERTIES,
    halftoneCmykUniforms,
} from "./effect.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The properties of the CMYK halftone besides its declared ones: the shader's and its image. */
export type HalftoneCmykAttributes = Omit<
    ShaderProperties,
    "fragmentShader" | "uniforms" | "speed" | "frame"
> & {
    /** The image it draws over, none for its background alone. */
    readonly image?: string | HTMLImageElement | null;
};

/** Draw the CMYK halftone. */
export const HalftoneCmyk = defineComponent({
    name: "halftoneCmyk",
    properties: HALFTONE_CMYK_PROPERTIES,
    render(
        properties: RenderProperties<HalftoneCmykAttributes, typeof HALFTONE_CMYK_PROPERTIES>,
    ): JSX.Element {
        // pass what the shader takes besides the effect's properties
        const rest = omit(
            properties,
            ...SIZING_KEYS,
            "image",
            "colorBack",
            "colorC",
            "colorM",
            "colorY",
            "colorK",
            "size",
            "contrast",
            "softness",
            "grainSize",
            "grainMixer",
            "grainOverlay",
            "gridNoise",
            "floodC",
            "floodM",
            "floodY",
            "floodK",
            "gainC",
            "gainM",
            "gainY",
            "gainK",
            "type",
        );

        return (
            <Shader
                fragmentShader={HALFTONE_CMYK_FRAGMENT}
                uniforms={halftoneCmykUniforms(properties)}
                {...rest}
            />
        );
    },
});

/** The properties of the CMYK halftone, the shader's included. */
export type HalftoneCmykProperties = ComponentProperties<typeof HalftoneCmyk>;
