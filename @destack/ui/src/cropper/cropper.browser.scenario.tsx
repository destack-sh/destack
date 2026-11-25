import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { cropperPhoto, cropperProfile } from "./cropper.example.tsx";
import { Cropper } from "./cropper.tsx";

/** The frame around the part. */
const frame = { css: "[data-slot=cropper-selection]" };

/** Read the size of the part. */
const size = { kind: "attribute", target: frame, name: "aria-description" } as const;

/** Resize a part by the keys, and turn the image. */
export const cropperResize = defineScenario({
    of: Cropper,
    interaction: viewInteraction,
    name: "resize",
    description: "resize a part by the keys, and turn the image",
    given: { examples: [cropperPhoto] },
    parameters: {},
    when: [
        {
            step: { action: "focus", target: frame },
            then: [{ observe: size, is: "⁨200⁩ by ⁨120⁩ pixels" }],
        },
        {
            step: { action: "press", key: "Alt+Shift+ArrowRight" },
            then: [{ observe: size, is: "⁨236⁩ by ⁨120⁩ pixels" }],
        },
        {
            step: { action: "click", target: { role: "button", name: "Rotate right" } },
            then: [{ observe: size, is: "⁨120⁩ by ⁨236⁩ pixels" }],
        },
    ],
});

/** Zoom a picture below its frame by the keys, as far as it goes. */
export const cropperZoom = defineScenario({
    of: Cropper,
    interaction: viewInteraction,
    name: "zoom",
    description: "zoom a picture below its frame by the keys, as far as it goes",
    given: { examples: [cropperProfile] },
    parameters: {},
    when: [
        {
            step: { action: "focus", target: frame },
            then: [{ observe: size, is: "⁨360⁩ by ⁨360⁩ pixels" }],
        },
        {
            step: { action: "press", key: "+" },
            then: [{ observe: size, is: "⁨327⁩ by ⁨327⁩ pixels" }],
        },
        { step: { action: "press", key: "-" } },
        {
            step: { action: "press", key: "-" },
            then: [{ observe: size, is: "⁨360⁩ by ⁨360⁩ pixels" }],
        },
    ],
});
