import { t } from "@destack/locale";
import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { Checkbox } from "./checkbox.tsx";
import { checkboxDefault } from "./checkbox.example.tsx";

/** The checkbox of the terms example. */
const terms = { role: "checkbox", name: "Accept the terms" };

/** Check a checkbox with Space and uncheck it from its label. */
export const checkboxCheckWithSpaceAndLabel = defineScenario({
    of: Checkbox,
    interaction: viewInteraction,
    name: "checkWithSpaceAndLabel",
    description: t`Check a checkbox with Space and uncheck it by pressing its label.`,
    given: { examples: [checkboxDefault] },
    parameters: {},
    when: [
        {
            step: { action: "focus", target: terms },
            then: [{ observe: { kind: "state", target: terms, state: "checked" }, is: false }],
        },
        {
            step: { action: "press", key: " " },
            then: [{ observe: { kind: "state", target: terms, state: "checked" }, is: true }],
        },
        {
            step: { action: "click", target: { text: "Accept the terms" } },
            then: [{ observe: { kind: "state", target: terms, state: "checked" }, is: false }],
        },
    ],
});
