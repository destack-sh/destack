import { type JSX, omit } from "@destack/view";
import { Shader, type ShaderProperties } from "../shader/shader.tsx";
import { SIZING_KEYS } from "../shader/sizing.ts";
import {
    STATIC_RADIAL_GRADIENT_FRAGMENT,
    STATIC_RADIAL_GRADIENT_PROPERTIES,
    staticRadialGradientUniforms,
} from "./effect.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The properties of the static radial gradient besides its declared ones: the shader's own. */
export type StaticRadialGradientAttributes = Omit<
    ShaderProperties,
    "fragmentShader" | "uniforms" | "speed" | "frame"
>;

/** Draw the static radial gradient. */
export const StaticRadialGradient = defineComponent({
    name: "staticRadialGradient",
    properties: STATIC_RADIAL_GRADIENT_PROPERTIES,
    render(
        properties: RenderProperties<
            StaticRadialGradientAttributes,
            typeof STATIC_RADIAL_GRADIENT_PROPERTIES
        >,
    ): JSX.Element {
        // pass what the shader takes besides the effect's properties
        const rest = omit(
            properties,
            ...SIZING_KEYS,
            "colorBack",
            "colors",
            "radius",
            "focalDistance",
            "focalAngle",
            "falloff",
            "mixing",
            "distortion",
            "distortionShift",
            "distortionFreq",
            "grainMixer",
            "grainOverlay",
        );

        return (
            <Shader
                fragmentShader={STATIC_RADIAL_GRADIENT_FRAGMENT}
                uniforms={staticRadialGradientUniforms(properties)}
                {...rest}
            />
        );
    },
});

/** The properties of the static radial gradient, the shader's included. */
export type StaticRadialGradientProperties = ComponentProperties<typeof StaticRadialGradient>;
