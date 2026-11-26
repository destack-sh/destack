import { t } from "@destack/locale";
import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { tabsTriggerDisabled, tabsManual } from "./tabs.example.tsx";
import { Tabs } from "./tabs.tsx";

/** Select tabs with the arrow keys. */
export const tabsSelectWithArrowKeys = defineScenario({
    of: Tabs,
    interaction: viewInteraction,
    name: "selectWithArrowKeys",
    description: t`Select tabs as the arrow keys move the focus, skipping disabled tabs and wrapping.`,
    given: { examples: [tabsTriggerDisabled] },
    parameters: {},
    when: [
        {
            step: { action: "focus", target: { role: "tab", name: "Edit" } },
            then: [
                { observe: { kind: "focused" }, is: "Edit" },
                { observe: { kind: "name", target: { role: "tab", selected: true } }, is: "Edit" },
            ],
        },
        {
            step: { action: "press", key: "ArrowRight" },
            then: [
                { observe: { kind: "focused" }, is: "Preview" },
                {
                    observe: { kind: "name", target: { role: "tab", selected: true } },
                    is: "Preview",
                },
            ],
        },
        {
            step: { action: "press", key: "ArrowRight" },
            then: [
                { observe: { kind: "focused" }, is: "Edit" },
                { observe: { kind: "name", target: { role: "tab", selected: true } }, is: "Edit" },
            ],
        },
        {
            step: { action: "press", key: "ArrowLeft" },
            then: [
                { observe: { kind: "focused" }, is: "Preview" },
                {
                    observe: { kind: "name", target: { role: "tab", selected: true } },
                    is: "Preview",
                },
            ],
        },
        {
            step: { action: "press", key: "End" },
            then: [
                { observe: { kind: "focused" }, is: "Preview" },
                {
                    observe: { kind: "name", target: { role: "tab", selected: true } },
                    is: "Preview",
                },
            ],
        },
        {
            step: { action: "press", key: "Home" },
            then: [
                { observe: { kind: "focused" }, is: "Edit" },
                { observe: { kind: "name", target: { role: "tab", selected: true } }, is: "Edit" },
            ],
        },
    ],
});

/** Move between manual tabs without selecting them. */
export const tabsMoveWithoutSelecting = defineScenario({
    of: Tabs,
    interaction: viewInteraction,
    name: "moveWithoutSelecting",
    description: t`Move the focus without selecting in manual activation, selecting on click.`,
    given: { examples: [tabsManual] },
    parameters: {},
    when: [
        {
            step: { action: "focus", target: { role: "tab", name: "Edit" } },
            then: [
                { observe: { kind: "focused" }, is: "Edit" },
                { observe: { kind: "name", target: { role: "tab", selected: true } }, is: "Edit" },
                { observe: { kind: "text", target: { role: "tabpanel" } }, is: "Editor" },
            ],
        },
        {
            step: { action: "press", key: "ArrowRight" },
            then: [
                { observe: { kind: "focused" }, is: "Preview" },
                { observe: { kind: "name", target: { role: "tab", selected: true } }, is: "Edit" },
                { observe: { kind: "text", target: { role: "tabpanel" } }, is: "Editor" },
            ],
        },
        {
            step: { action: "click", target: { role: "tab", name: "Preview" } },
            then: [
                { observe: { kind: "focused" }, is: "Preview" },
                {
                    observe: { kind: "name", target: { role: "tab", selected: true } },
                    is: "Preview",
                },
                { observe: { kind: "text", target: { role: "tabpanel" } }, is: "Rendered" },
            ],
        },
    ],
});
