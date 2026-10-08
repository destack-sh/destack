import { type JSX, merge, omit } from "@destack/view";
import { Shader } from "../shader/shader.tsx";
import { SIZING_KEYS } from "../shader/sizing.ts";
import {
    METABALLS_DEFAULTS,
    METABALLS_FRAGMENT,
    type MetaballsProperties,
    metaballsUniforms,
} from "./effect.ts";

/** Draw the metaballs. */
export function Metaballs(properties: MetaballsProperties): JSX.Element {
    // fill unset options from the defaults and pass the rest to the shader
    const options = merge(METABALLS_DEFAULTS, properties);
    const rest = omit(options, ...SIZING_KEYS, "colorBack", "colors", "count", "size");

    return <Shader fragment={METABALLS_FRAGMENT} uniforms={metaballsUniforms(options)} {...rest} />;
}
