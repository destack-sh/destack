import { type JSX, merge, omit } from "@destack/view";
import { Shader } from "../shader/shader.tsx";
import { SIZING_KEYS } from "../shader/sizing.ts";
import {
    DOT_GRID_DEFAULTS,
    DOT_GRID_FRAGMENT,
    type DotGridProperties,
    dotGridUniforms,
} from "./effect.ts";

/** Draw the dot grid. */
export function DotGrid(properties: DotGridProperties): JSX.Element {
    // fill unset options from the defaults and pass the rest to the shader
    const options = merge(DOT_GRID_DEFAULTS, properties);
    const rest = omit(
        options,
        ...SIZING_KEYS,
        "colorBack",
        "colorFill",
        "colorStroke",
        "size",
        "gapX",
        "gapY",
        "strokeWidth",
        "sizeRange",
        "opacityRange",
        "shape",
    );

    return <Shader fragment={DOT_GRID_FRAGMENT} uniforms={dotGridUniforms(options)} {...rest} />;
}
