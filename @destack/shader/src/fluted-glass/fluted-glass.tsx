import { type JSX, omit } from "@destack/view";
import { Shader, type ShaderProperties } from "../shader/shader.tsx";
import { SIZING_KEYS } from "../shader/sizing.ts";
import { FLUTED_GLASS_FRAGMENT, FLUTED_GLASS_PROPERTIES, flutedGlassUniforms } from "./effect.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The properties of the fluted glass besides its declared ones: the shader's and its image. */
export type FlutedGlassAttributes = Omit<
    ShaderProperties,
    "fragmentShader" | "uniforms" | "speed" | "frame"
> & {
    /** The image it draws over, none for its background alone. */
    readonly image?: string | HTMLImageElement | null;
};

/** Draw the fluted glass. */
export const FlutedGlass = defineComponent({
    name: "flutedGlass",
    properties: FLUTED_GLASS_PROPERTIES,
    render(
        properties: RenderProperties<FlutedGlassAttributes, typeof FLUTED_GLASS_PROPERTIES>,
    ): JSX.Element {
        // pass what the shader takes besides the effect's properties
        const rest = omit(
            properties,
            ...SIZING_KEYS,
            "image",
            "colorBack",
            "colorShadow",
            "colorHighlight",
            "shadows",
            "size",
            "angle",
            "distortionShape",
            "highlights",
            "shape",
            "distortion",
            "shift",
            "blur",
            "edges",
            "stretch",
            "marginLeft",
            "marginRight",
            "marginTop",
            "marginBottom",
            "grainMixer",
            "grainOverlay",
        );

        return (
            <Shader
                fragmentShader={FLUTED_GLASS_FRAGMENT}
                uniforms={flutedGlassUniforms(properties)}
                {...rest}
            />
        );
    },
});

/** The properties of the fluted glass, the shader's included. */
export type FlutedGlassProperties = ComponentProperties<typeof FlutedGlass>;
