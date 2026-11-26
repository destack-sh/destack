import { type JSX, omit } from "@destack/view";
import { Shader, type ShaderProperties } from "../shader/shader.tsx";
import { SIZING_KEYS } from "../shader/sizing.ts";
import {
    PULSING_BORDER_FRAGMENT,
    PULSING_BORDER_PROPERTIES,
    pulsingBorderUniforms,
} from "./effect.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The properties of the pulsing border besides its declared ones: the shader's own. */
export type PulsingBorderAttributes = Omit<
    ShaderProperties,
    "fragmentShader" | "uniforms" | "speed" | "frame"
>;

/** Draw the pulsing border. */
export const PulsingBorder = defineComponent({
    name: "pulsingBorder",
    properties: PULSING_BORDER_PROPERTIES,
    render(
        properties: RenderProperties<PulsingBorderAttributes, typeof PULSING_BORDER_PROPERTIES>,
    ): JSX.Element {
        // pass what the shader takes besides the effect's properties
        const rest = omit(
            properties,
            ...SIZING_KEYS,
            "colorBack",
            "colors",
            "roundness",
            "thickness",
            "marginLeft",
            "marginRight",
            "marginTop",
            "marginBottom",
            "aspectRatio",
            "softness",
            "intensity",
            "bloom",
            "spots",
            "spotSize",
            "pulse",
            "smoke",
            "smokeSize",
        );

        return (
            <Shader
                fragmentShader={PULSING_BORDER_FRAGMENT}
                uniforms={pulsingBorderUniforms(properties)}
                {...rest}
            />
        );
    },
});

/** The properties of the pulsing border, the shader's included. */
export type PulsingBorderProperties = ComponentProperties<typeof PulsingBorder>;
