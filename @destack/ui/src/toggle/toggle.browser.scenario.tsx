import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { Toggle } from "./toggle.tsx";
import { toggleBold } from "./toggle.example.tsx";

/** The toggle of the bold example. */
const bold = { role: "button", name: "Bold" };

/** Press a toggle on with Space and off with Enter. */
export const togglePressWithKeys = defineScenario({
    of: Toggle,
    interaction: viewInteraction,
    name: "press-with-keys",
    description: "press a toggle on with Space and off again with Enter",
    given: { examples: [toggleBold] },
    parameters: {},
    when: [
        {
            step: { action: "focus", target: bold },
            then: [{ observe: { kind: "state", target: bold, state: "pressed" }, is: false }],
        },
        {
            step: { action: "press", key: " " },
            then: [{ observe: { kind: "state", target: bold, state: "pressed" }, is: true }],
        },
        {
            step: { action: "press", key: "Enter" },
            then: [{ observe: { kind: "state", target: bold, state: "pressed" }, is: false }],
        },
    ],
});
