import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { tagFieldNote } from "./tag-field.example.tsx";
import { TagField } from "./tag-field.tsx";

/** The tags. */
const tags = { css: "[data-slot=tag-field-item]" };

/** The text of the next tag. */
const input = { css: "[data-slot=tag-field-input]" };

/** Add a tag by Enter, go to it by Backspace, and remove it by another. */
export const tagFieldType = defineScenario({
    of: TagField,
    interaction: viewInteraction,
    name: "type",
    description: "add a tag by Enter, go to it by Backspace, and remove it by another",
    given: { examples: [tagFieldNote] },
    parameters: {},
    when: [
        { step: { action: "click", target: input } },
        { step: { action: "fill", target: input, value: "food" } },
        {
            step: { action: "press", key: "Enter" },
            then: [
                {
                    observe: { kind: "texts", target: tags },
                    is: ["travel", "lisbon", "spring", "food"],
                },
                { observe: { kind: "value", target: input }, is: "" },
            ],
        },
        { step: { action: "press", key: "Backspace" } },
        {
            step: { action: "press", key: "Backspace" },
            then: [
                { observe: { kind: "texts", target: tags }, is: ["travel", "lisbon", "spring"] },
                { observe: { kind: "text", target: { role: "status" } }, is: "Removed ⁨food⁩" },
            ],
        },
    ],
});
