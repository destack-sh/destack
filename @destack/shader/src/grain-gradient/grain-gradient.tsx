import { type JSX, omit } from "@destack/view";
import { Shader, type ShaderProperties } from "../shader/shader.tsx";
import { SIZING_KEYS } from "../shader/sizing.ts";
import {
    GRAIN_GRADIENT_FRAGMENT,
    GRAIN_GRADIENT_PROPERTIES,
    grainGradientUniforms,
} from "./effect.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The properties of the grain gradient besides its declared ones: the shader's own. */
export type GrainGradientAttributes = Omit<
    ShaderProperties,
    "fragmentShader" | "uniforms" | "speed" | "frame"
>;

/** Draw the grain gradient. */
export const GrainGradient = defineComponent({
    name: "grainGradient",
    properties: GRAIN_GRADIENT_PROPERTIES,
    render(
        properties: RenderProperties<GrainGradientAttributes, typeof GRAIN_GRADIENT_PROPERTIES>,
    ): JSX.Element {
        // pass what the shader takes besides the effect's properties
        const rest = omit(
            properties,
            ...SIZING_KEYS,
            "colorBack",
            "colors",
            "softness",
            "intensity",
            "noise",
            "shape",
        );

        return (
            <Shader
                fragmentShader={GRAIN_GRADIENT_FRAGMENT}
                uniforms={grainGradientUniforms(properties)}
                {...rest}
            />
        );
    },
});

/** The properties of the grain gradient, the shader's included. */
export type GrainGradientProperties = ComponentProperties<typeof GrainGradient>;
