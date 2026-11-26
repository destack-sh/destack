import { type JSX, omit } from "@destack/view";
import { Shader, type ShaderProperties } from "../shader/shader.tsx";
import { SIZING_KEYS } from "../shader/sizing.ts";
import { VORONOI_FRAGMENT, VORONOI_PROPERTIES, voronoiUniforms } from "./effect.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The properties of the voronoi besides its declared ones: the shader's own. */
export type VoronoiAttributes = Omit<
    ShaderProperties,
    "fragmentShader" | "uniforms" | "speed" | "frame"
>;

/** Draw the voronoi. */
export const Voronoi = defineComponent({
    name: "voronoi",
    properties: VORONOI_PROPERTIES,
    render(
        properties: RenderProperties<VoronoiAttributes, typeof VORONOI_PROPERTIES>,
    ): JSX.Element {
        // pass what the shader takes besides the effect's properties
        const rest = omit(
            properties,
            ...SIZING_KEYS,
            "colors",
            "stepsPerColor",
            "colorGlow",
            "colorGap",
            "distortion",
            "gap",
            "glow",
        );

        return (
            <Shader
                fragmentShader={VORONOI_FRAGMENT}
                uniforms={voronoiUniforms(properties)}
                {...rest}
            />
        );
    },
});

/** The properties of the voronoi, the shader's included. */
export type VoronoiProperties = ComponentProperties<typeof Voronoi>;
