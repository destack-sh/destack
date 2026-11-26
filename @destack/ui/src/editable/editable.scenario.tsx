import { t } from "@destack/locale";
import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { editablePage } from "./editable.example.tsx";
import { Editable } from "./editable.tsx";

/** The title as it is read. */
const title = { css: "h1 [data-slot=editable-preview]" };

/** The title as it is written. */
const input = { css: "h1 [data-slot=editable-input]" };

/** Change a title by the keys, leave a change, and put an empty one back. */
export const editableRename = defineScenario({
    of: Editable,
    interaction: viewInteraction,
    name: "rename",
    description: t`Change a title by the keys, leave a change, and put an empty one back.`,
    given: { examples: [editablePage] },
    parameters: {},
    when: [
        { step: { action: "focus", target: title } },
        { step: { action: "press", key: "Enter" } },
        { step: { action: "fill", target: input, value: "Autumn in Porto" } },
        {
            step: { action: "press", key: "Enter" },
            then: [{ observe: { kind: "text", target: title }, is: "Autumn in Porto" }],
        },
        { step: { action: "press", key: "Enter" } },
        { step: { action: "fill", target: input, value: "Winter" } },
        {
            step: { action: "press", key: "Escape" },
            then: [{ observe: { kind: "text", target: title }, is: "Autumn in Porto" }],
        },
        { step: { action: "press", key: "Enter" } },
        { step: { action: "fill", target: input, value: "" } },
        {
            step: { action: "press", key: "Enter" },
            then: [{ observe: { kind: "text", target: title }, is: "Autumn in Porto" }],
        },
    ],
});
