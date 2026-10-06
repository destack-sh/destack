import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";
import { accordionHelpQuestionsDisabled } from "./accordion.example.tsx";
import { Accordion } from "./accordion.tsx";

/** Move between an accordion's triggers with the arrow keys, Home and End, skipping disabled ones. */
export const accordionMoveBetweenTriggers = defineScenario({
    of: Accordion,
    interaction: viewInteraction,
    name: "move-between-triggers",
    description:
        "move the focus between an accordion's triggers with the arrow keys, Home and End, skipping a disabled one and wrapping",
    given: { examples: [accordionHelpQuestionsDisabled] },
    when: [
        { action: "focus", target: { css: "details:first-child > [data-slot=accordion-trigger]" } },
        { action: "press", key: "ArrowDown" },
        { action: "press", key: "End" },
        { action: "press", key: "Home" },
    ],
    then: {
        observe: { focused: { kind: "focused" } },
        each: [
            { focused: "Who can see my notes?" },
            { focused: "Can I work offline?" },
            { focused: "Can I work offline?" },
            { focused: "Who can see my notes?" },
        ],
    },
});
