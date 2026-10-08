import { type JSX, merge, omit } from "@destack/view";
import { Shader } from "../shader/shader.tsx";
import { SIZING_KEYS } from "../shader/sizing.ts";
import {
    PERLIN_NOISE_DEFAULTS,
    PERLIN_NOISE_FRAGMENT,
    type PerlinNoiseProperties,
    perlinNoiseUniforms,
} from "./effect.ts";

/** Draw the perlin noise. */
export function PerlinNoise(properties: PerlinNoiseProperties): JSX.Element {
    // fill unset options from the defaults and pass the rest to the shader
    const options = merge(PERLIN_NOISE_DEFAULTS, properties);
    const rest = omit(
        options,
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
            uniforms={perlinNoiseUniforms(options)}
            {...rest}
        />
    );
}
