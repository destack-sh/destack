import { t } from "@destack/locale";
import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { overflowNarrow, overflowTabs } from "./overflow.example.tsx";
import { Overflow } from "./overflow.tsx";

/** The button that opens the menu of what does not fit. */
const more = { css: "[data-slot=overflow-menu-trigger]" };

/** Hide what does not fit, and show what is chosen from the menu in its place. */
export const overflowChooseHidden = defineScenario({
    of: Overflow,
    interaction: viewInteraction,
    name: "chooseHidden",
    description: t`Hide what does not fit, and show what is chosen from the menu in its place.`,
    given: { examples: [overflowNarrow] },
    parameters: {},
    when: [
        {
            step: { action: "focus", target: { role: "tab", name: "Overview" } },
            then: [
                {
                    observe: { kind: "visible", target: { role: "tab", name: "Roadmap" } },
                    is: true,
                },
                { observe: { kind: "visible", target: more }, is: true },
                {
                    observe: { kind: "count", target: { css: "[role=tab]:not([hidden])" } },
                    is: 3,
                },
            ],
        },
        {
            step: { action: "click", target: more },
            then: [
                {
                    observe: { kind: "texts", target: { css: "[role=menuitem]" } },
                    is: ["Releases", "Members", "Billing", "Settings"],
                },
            ],
        },
        {
            step: { action: "click", target: { role: "menuitem", name: "Billing" } },
        },
        {
            step: { action: "pause", milliseconds: 200 },
            then: [
                { observe: { kind: "focused" }, is: "Billing" },
                {
                    observe: { kind: "name", target: { role: "tab", selected: true } },
                    is: "Billing",
                },
                { observe: { kind: "text", target: { role: "tabpanel" } }, is: "Billing" },
                {
                    observe: { kind: "visible", target: { role: "tab", name: "Billing" } },
                    is: true,
                },
            ],
        },
    ],
});

/** Show no menu where everything fits. */
export const overflowShowAll = defineScenario({
    of: Overflow,
    interaction: viewInteraction,
    name: "showAll",
    description: t`Show no menu where everything fits.`,
    given: { examples: [overflowTabs] },
    parameters: {},
    when: [
        {
            step: { action: "focus", target: { role: "tab", name: "Overview" } },
            then: [
                { observe: { kind: "count", target: { css: "[role=tab]:not([hidden])" } }, is: 7 },
                { observe: { kind: "visible", target: more }, is: false },
            ],
        },
    ],
});
