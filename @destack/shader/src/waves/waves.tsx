import { type JSX, omit } from "@destack/view";
import { Shader, type ShaderProperties } from "../shader/shader.tsx";
import { SIZING_KEYS } from "../shader/sizing.ts";
import { WAVES_FRAGMENT, WAVES_PROPERTIES, wavesUniforms } from "./effect.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The properties of the waves besides its declared ones: the shader's own. */
export type WavesAttributes = Omit<
    ShaderProperties,
    "fragmentShader" | "uniforms" | "speed" | "frame"
>;

/** Draw the waves. */
export const Waves = defineComponent({
    name: "waves",
    properties: WAVES_PROPERTIES,
    render(properties: RenderProperties<WavesAttributes, typeof WAVES_PROPERTIES>): JSX.Element {
        // pass what the shader takes besides the effect's properties
        const rest = omit(
            properties,
            ...SIZING_KEYS,
            "colorFront",
            "colorBack",
            "shape",
            "frequency",
            "amplitude",
            "spacing",
            "proportion",
            "softness",
        );

        return (
            <Shader
                fragmentShader={WAVES_FRAGMENT}
                uniforms={wavesUniforms(properties)}
                {...rest}
            />
        );
    },
});

/** The properties of the waves, the shader's included. */
export type WavesProperties = ComponentProperties<typeof Waves>;
