import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import { Accordion, AccordionContent, AccordionItem, AccordionTrigger } from "./accordion.tsx";

/** A help page's questions, one answer open at a time. */
export const accordionHelpQuestions = defineExample({
    of: Accordion,
    name: "helpQuestions",
    description: t`A help page's questions, one answer open at a time.`,
    properties: {
        defaultValue: "sharing",
    },
    render: (properties) => (
        <Accordion {...properties}>
            <AccordionItem value="sharing">
                <AccordionTrigger>Who can see my notes?</AccordionTrigger>
                <AccordionContent>Only the people you share a notebook with.</AccordionContent>
            </AccordionItem>
            <AccordionItem value="offline">
                <AccordionTrigger>Can I work offline?</AccordionTrigger>
                <AccordionContent>Yes, changes sync when you are back online.</AccordionContent>
            </AccordionItem>
        </Accordion>
    ),
});

/** A help page's questions with every answer open. */
export const accordionHelpQuestionsExpanded = defineExample({
    of: Accordion,
    name: "helpQuestionsExpanded",
    description: t`A help page's questions with every answer open.`,
    render: (properties) => (
        <Accordion
            multiple
            defaultValue={["sharing", "offline"]}
            orientation={properties.orientation}
            disabled={properties.disabled}
        >
            <AccordionItem value="sharing">
                <AccordionTrigger>Who can see my notes?</AccordionTrigger>
                <AccordionContent>Only the people you share a notebook with.</AccordionContent>
            </AccordionItem>
            <AccordionItem value="offline">
                <AccordionTrigger>Can I work offline?</AccordionTrigger>
                <AccordionContent>Yes, changes sync when you are back online.</AccordionContent>
            </AccordionItem>
        </Accordion>
    ),
});

/** A help page's questions, the billing question set on its own. */
export const accordionItemDefault = defineExample({
    of: AccordionItem,
    name: "default",
    description: t`A help page's questions, the billing question set on its own.`,
    properties: { disabled: true },
    render: (properties) => (
        <Accordion>
            <AccordionItem value="sharing">
                <AccordionTrigger>Who can see my notes?</AccordionTrigger>
                <AccordionContent>Only the people you share a notebook with.</AccordionContent>
            </AccordionItem>
            <AccordionItem value="billing" {...properties}>
                <AccordionTrigger>How do I pay?</AccordionTrigger>
                <AccordionContent>Your workspace owner handles billing.</AccordionContent>
            </AccordionItem>
            <AccordionItem value="offline">
                <AccordionTrigger>Can I work offline?</AccordionTrigger>
                <AccordionContent>Yes, changes sync when you are back online.</AccordionContent>
            </AccordionItem>
        </Accordion>
    ),
});
