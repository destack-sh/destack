import { type JSX, merge, omit } from "@destack/view";
import { Shader } from "../shader/shader.tsx";
import { SIZING_KEYS } from "../shader/sizing.ts";
import {
    VORONOI_DEFAULTS,
    VORONOI_FRAGMENT,
    type VoronoiProperties,
    voronoiUniforms,
} from "./effect.ts";

/** Draw the voronoi. */
export function Voronoi(properties: VoronoiProperties): JSX.Element {
    // fill unset options from the defaults and pass the rest to the shader
    const options = merge(VORONOI_DEFAULTS, properties);
    const rest = omit(
        options,
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
        <Shader fragmentShader={VORONOI_FRAGMENT} uniforms={voronoiUniforms(options)} {...rest} />
    );
}
