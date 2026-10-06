import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { tooltipArchiveButton } from "./tooltip.example.tsx";
import { Tooltip } from "./tooltip.tsx";

/** Show a tooltip on focus and hide it on Escape. */
export const tooltipShowOnFocus = defineScenario({
    of: Tooltip,
    interaction: viewInteraction,
    name: "show-on-focus",
    description: "show a tooltip at once while its trigger has the focus and hide it on Escape",
    given: { examples: [tooltipArchiveButton] },
    when: [
        { action: "focus", target: { role: "button", name: "Archive" } },
        { action: "press", key: "Escape" },
    ],
    then: {
        observe: { tooltip: { kind: "visible", target: { role: "tooltip" } } },
        each: [{ tooltip: true }, { tooltip: false }],
    },
});
