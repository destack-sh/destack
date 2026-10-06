import { defineExample } from "@destack/package/declare";
import { Accordion, AccordionContent, AccordionItem, AccordionTrigger } from "./accordion.tsx";

/** A help page's questions, one answer open at a time. */
export const accordionHelpQuestions = defineExample({
    of: Accordion,
    name: "help-questions",
    description: "a help page's questions, one answer open at a time",
    render: () => (
        <Accordion>
            <AccordionItem open>
                <AccordionTrigger>Who can see my notes?</AccordionTrigger>
                <AccordionContent>Only the people you share a notebook with.</AccordionContent>
            </AccordionItem>
            <AccordionItem>
                <AccordionTrigger>Can I work offline?</AccordionTrigger>
                <AccordionContent>Yes, changes sync when you are back online.</AccordionContent>
            </AccordionItem>
        </Accordion>
    ),
});

/** A help page's questions with every answer open. */
export const accordionHelpQuestionsExpanded = defineExample({
    of: Accordion,
    name: "help-questions-expanded",
    description: "a help page's questions with every answer open",
    render: () => (
        <Accordion type="multiple">
            <AccordionItem open>
                <AccordionTrigger>Who can see my notes?</AccordionTrigger>
                <AccordionContent>Only the people you share a notebook with.</AccordionContent>
            </AccordionItem>
            <AccordionItem open>
                <AccordionTrigger>Can I work offline?</AccordionTrigger>
                <AccordionContent>Yes, changes sync when you are back online.</AccordionContent>
            </AccordionItem>
        </Accordion>
    ),
});
