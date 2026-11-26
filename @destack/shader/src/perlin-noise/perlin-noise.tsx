import { type JSX, omit } from "@destack/view";
import { Shader, type ShaderProperties } from "../shader/shader.tsx";
import { SIZING_KEYS } from "../shader/sizing.ts";
import { PERLIN_NOISE_FRAGMENT, PERLIN_NOISE_PROPERTIES, perlinNoiseUniforms } from "./effect.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The properties of the perlin noise besides its declared ones: the shader's own. */
export type PerlinNoiseAttributes = Omit<
    ShaderProperties,
    "fragmentShader" | "uniforms" | "speed" | "frame"
>;

/** Draw the perlin noise. */
export const PerlinNoise = defineComponent({
    name: "perlinNoise",
    properties: PERLIN_NOISE_PROPERTIES,
    render(
        properties: RenderProperties<PerlinNoiseAttributes, typeof PERLIN_NOISE_PROPERTIES>,
    ): JSX.Element {
        // pass what the shader takes besides the effect's properties
        const rest = omit(
            properties,
            ...SIZING_KEYS,
            "colorBack",
            "colorFront",
            "proportion",
            "softness",
            "octaveCount",
            "persistence",
            "lacunarity",
        );

        return (
            <Shader
                fragmentShader={PERLIN_NOISE_FRAGMENT}
                uniforms={perlinNoiseUniforms(properties)}
                {...rest}
            />
        );
    },
});

/** The properties of the perlin noise, the shader's included. */
export type PerlinNoiseProperties = ComponentProperties<typeof PerlinNoise>;
