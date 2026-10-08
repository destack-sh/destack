import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { coverImagePageCover } from "./cover-image.example.tsx";
import { CoverImage } from "./cover-image.tsx";

/** Reposition a cover with the keys. */
export const coverImageRepositionWithKeys = defineScenario({
    of: CoverImage,
    interaction: viewInteraction,
    name: "reposition-with-keys",
    description:
        "move a cover's position a step with the arrow keys, a page with Page Up and Page Down, and to the top or bottom with Home and End",
    given: { examples: [coverImagePageCover] },
    when: [
        { action: "focus", target: { role: "slider", name: "Reposition the cover" } },
        { action: "press", key: "ArrowDown" },
        { action: "press", key: "PageDown" },
        { action: "press", key: "ArrowUp" },
        { action: "press", key: "End" },
        { action: "press", key: "PageUp" },
        { action: "press", key: "Home" },
    ],
    then: {
        observe: {
            position: {
                kind: "value",
                target: { role: "slider", name: "Reposition the cover" },
            },
        },
        each: [
            { position: "33" },
            { position: "38" },
            { position: "58" },
            { position: "53" },
            { position: "100" },
            { position: "80" },
            { position: "0" },
        ],
    },
});
