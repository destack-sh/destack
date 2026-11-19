import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { accordionHelpQuestionsDisabled } from "./accordion.example.tsx";
import { Accordion } from "./accordion.tsx";

/** Move between an accordion's triggers with the arrow keys, Home and End. */
export const accordionMoveBetweenTriggers = defineScenario({
    of: Accordion,
    interaction: viewInteraction,
    name: "move-between-triggers",
    description:
        "move the focus between an accordion's triggers with the arrow keys, Home and End, skipping a disabled one and wrapping",
    given: { examples: [accordionHelpQuestionsDisabled] },
    parameters: {},
    when: [
        {
            step: {
                action: "focus",
                target: { css: "details:first-child > [data-slot=accordion-trigger]" },
            },
            then: [{ observe: { kind: "focused" }, is: "Who can see my notes?" }],
        },
        {
            step: { action: "press", key: "ArrowDown" },
            then: [{ observe: { kind: "focused" }, is: "Can I work offline?" }],
        },
        {
            step: { action: "press", key: "End" },
            then: [{ observe: { kind: "focused" }, is: "Can I work offline?" }],
        },
        {
            step: { action: "press", key: "Home" },
            then: [{ observe: { kind: "focused" }, is: "Who can see my notes?" }],
        },
    ],
});
