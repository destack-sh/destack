import { type JSX, omit } from "@destack/view";
import { Shader, type ShaderProperties } from "../shader/shader.tsx";
import { SIZING_KEYS } from "../shader/sizing.ts";
import {
    SIMPLEX_NOISE_FRAGMENT,
    SIMPLEX_NOISE_PROPERTIES,
    simplexNoiseUniforms,
} from "./effect.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The properties of the simplex noise besides its declared ones: the shader's own. */
export type SimplexNoiseAttributes = Omit<
    ShaderProperties,
    "fragmentShader" | "uniforms" | "speed" | "frame"
>;

/** Draw the simplex noise. */
export const SimplexNoise = defineComponent({
    name: "simplexNoise",
    properties: SIMPLEX_NOISE_PROPERTIES,
    render(
        properties: RenderProperties<SimplexNoiseAttributes, typeof SIMPLEX_NOISE_PROPERTIES>,
    ): JSX.Element {
        // pass what the shader takes besides the effect's properties
        const rest = omit(properties, ...SIZING_KEYS, "colors", "stepsPerColor", "softness");

        return (
            <Shader
                fragmentShader={SIMPLEX_NOISE_FRAGMENT}
                uniforms={simplexNoiseUniforms(properties)}
                {...rest}
            />
        );
    },
});

/** The properties of the simplex noise, the shader's included. */
export type SimplexNoiseProperties = ComponentProperties<typeof SimplexNoise>;
