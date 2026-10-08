import { type JSX, merge, omit } from "@destack/view";
import { Shader } from "../shader/shader.tsx";
import { SIZING_KEYS } from "../shader/sizing.ts";
import {
    IMAGE_DITHERING_DEFAULTS,
    IMAGE_DITHERING_FRAGMENT,
    type ImageDitheringProperties,
    imageDitheringUniforms,
} from "./effect.ts";

/** Draw the image dithering. */
export function ImageDithering(properties: ImageDitheringProperties): JSX.Element {
    // fill unset options from the defaults and pass the rest to the shader
    const options = merge(IMAGE_DITHERING_DEFAULTS, properties);
    const rest = omit(
        options,
        ...SIZING_KEYS,
        "image",
        "colorFront",
        "colorBack",
        "colorHighlight",
        "type",
        "size",
        "colorSteps",
        "originalColors",
        "inverted",
    );

    return (
        <Shader
            fragment={IMAGE_DITHERING_FRAGMENT}
            uniforms={imageDitheringUniforms(options)}
            {...rest}
        />
    );
}
