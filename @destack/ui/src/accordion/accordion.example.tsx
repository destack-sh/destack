import type { JSX } from "@solidjs/web";
import { Accordion, AccordionContent, AccordionItem, AccordionTrigger } from "./accordion.tsx";

/** Show a help page's questions, one answer open at a time. */
export function AccordionExample(): JSX.Element {
    return (
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
    );
}
