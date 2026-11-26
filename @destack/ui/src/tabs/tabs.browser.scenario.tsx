import { t } from "@destack/locale";
import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { tabsFiles } from "./tabs.example.tsx";
import { Tabs } from "./tabs.tsx";

/** Select a tab that drags by pressing it, and go on by the keys. */
export const tabsPressDraggable = defineScenario({
    of: Tabs,
    interaction: viewInteraction,
    name: "pressDraggable",
    description: t`Select a tab that drags by pressing it, and go on by the keys.`,
    given: { examples: [tabsFiles] },
    parameters: {},
    when: [
        {
            step: { action: "click", target: { role: "tab", name: "plan.md" } },
            then: [
                {
                    observe: { kind: "name", target: { role: "tab", selected: true } },
                    is: "plan.md",
                },
                { observe: { kind: "text", target: { role: "tabpanel" } }, is: "plan.md" },
            ],
        },
        {
            step: { action: "press", key: "ArrowRight" },
            then: [
                { observe: { kind: "focused" }, is: "timeline.tsx" },
                {
                    observe: { kind: "name", target: { role: "tab", selected: true } },
                    is: "timeline.tsx",
                },
            ],
        },
    ],
});
