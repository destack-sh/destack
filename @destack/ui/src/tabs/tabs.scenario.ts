import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { tabsHistoryUnavailable, tabsManualActivation } from "./tabs.example.tsx";
import { Tabs } from "./tabs.tsx";

/** Select tabs with the arrow keys. */
export const tabsSelectWithArrowKeys = defineScenario({
    of: Tabs,
    interaction: viewInteraction,
    name: "select-with-arrow-keys",
    description:
        "select tabs as the arrow keys move the focus, skipping disabled tabs and wrapping",
    given: { examples: [tabsHistoryUnavailable] },
    when: [
        { action: "focus", target: { role: "tab", name: "Edit" } },
        { action: "press", key: "ArrowRight" },
        { action: "press", key: "ArrowRight" },
        { action: "press", key: "ArrowLeft" },
        { action: "press", key: "End" },
        { action: "press", key: "Home" },
    ],
    then: {
        observe: {
            focused: { kind: "focused" },
            selected: { kind: "name", target: { role: "tab", selected: true } },
        },
        each: [
            { focused: "Edit", selected: "Edit" },
            { focused: "Preview", selected: "Preview" },
            { focused: "Edit", selected: "Edit" },
            { focused: "Preview", selected: "Preview" },
            { focused: "Preview", selected: "Preview" },
            { focused: "Edit", selected: "Edit" },
        ],
    },
});

/** Move between manual tabs without selecting them. */
export const tabsMoveWithoutSelecting = defineScenario({
    of: Tabs,
    interaction: viewInteraction,
    name: "move-without-selecting",
    description: "move the focus without selecting in manual activation, selecting on click",
    given: { examples: [tabsManualActivation] },
    when: [
        { action: "focus", target: { role: "tab", name: "Edit" } },
        { action: "press", key: "ArrowRight" },
        { action: "click", target: { role: "tab", name: "Preview" } },
    ],
    then: {
        observe: {
            focused: { kind: "focused" },
            selected: { kind: "name", target: { role: "tab", selected: true } },
            panel: { kind: "text", target: { role: "tabpanel" } },
        },
        each: [
            { focused: "Edit", selected: "Edit", panel: "Editor" },
            { focused: "Preview", selected: "Edit", panel: "Editor" },
            { focused: "Preview", selected: "Preview", panel: "Rendered" },
        ],
    },
});
