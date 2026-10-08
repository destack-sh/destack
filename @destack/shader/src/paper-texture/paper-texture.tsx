import { type JSX, merge, omit } from "@destack/view";
import { Shader } from "../shader/shader.tsx";
import { SIZING_KEYS } from "../shader/sizing.ts";
import {
    PAPER_TEXTURE_DEFAULTS,
    PAPER_TEXTURE_FRAGMENT,
    type PaperTextureProperties,
    paperTextureUniforms,
} from "./effect.ts";

/** Draw the paper texture. */
export function PaperTexture(properties: PaperTextureProperties): JSX.Element {
    // fill unset options from the defaults and pass the rest to the shader
    const options = merge(PAPER_TEXTURE_DEFAULTS, properties);
    const rest = omit(
        options,
        ...SIZING_KEYS,
        "image",
        "colorBack",
        "colorPaper",
        "colorShadow",
        "blending",
        "distortion",
        "clip",
        "angle",
        "seed",
        "roughness",
        "roughnessSize",
        "roughnessRows",
        "fiber",
        "fiberSize",
        "folds",
        "foldSizeX",
        "foldSizeY",
        "foldOffsetX",
        "foldOffsetY",
        "wrinkles",
        "wrinkleSize",
        "crumples",
        "crumpleCount",
        "drops",
    );

    return (
        <Shader
            fragmentShader={PAPER_TEXTURE_FRAGMENT}
            uniforms={paperTextureUniforms(options)}
            {...rest}
        />
    );
}
