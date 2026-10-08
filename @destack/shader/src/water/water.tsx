import { type JSX, merge, omit } from "@destack/view";
import { Shader } from "../shader/shader.tsx";
import { SIZING_KEYS } from "../shader/sizing.ts";
import { WATER_DEFAULTS, WATER_FRAGMENT, type WaterProperties, waterUniforms } from "./effect.ts";

/** Draw the water. */
export function Water(properties: WaterProperties): JSX.Element {
    // fill unset options from the defaults and pass the rest to the shader
    const options = merge(WATER_DEFAULTS, properties);
    const rest = omit(
        options,
        ...SIZING_KEYS,
        "image",
        "colorBack",
        "colorHighlight",
        "highlights",
        "layering",
        "edges",
        "waves",
        "caustic",
        "size",
    );

    return <Shader fragment={WATER_FRAGMENT} uniforms={waterUniforms(options)} {...rest} />;
}
