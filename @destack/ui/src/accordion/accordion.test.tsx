import { expect, test } from "@destack/test";
import { Accordion, AccordionContent, AccordionItem, AccordionTrigger } from "./index.ts";
import { draw, markup } from "@destack/view/test";

test("name a single accordion's items alike so the platform keeps one open", () => {
    const container = draw(() => (
        <Accordion>
            <AccordionItem>
                <AccordionTrigger>Is it accessible?</AccordionTrigger>
                <AccordionContent>Yes.</AccordionContent>
            </AccordionItem>
            <AccordionItem open>
                <AccordionTrigger>Is it styled?</AccordionTrigger>
                <AccordionContent>Yes.</AccordionContent>
            </AccordionItem>
        </Accordion>
    ));
    const chevron =
        '<span><svg viewBox="0 0 256 256" fill="currentColor" width="1em" height="1em" aria-hidden="true"></svg></span>';
    expect(markup(container)).toBe(
        '<div data-slot="accordion">' +
            '<details name="id-1" data-slot="accordion-item">' +
            `<summary data-slot="accordion-trigger">Is it accessible?${chevron}</summary>` +
            '<div data-slot="accordion-content">Yes.</div></details>' +
            '<details name="id-1" data-slot="accordion-item" open="">' +
            `<summary data-slot="accordion-trigger">Is it styled?${chevron}</summary>` +
            '<div data-slot="accordion-content">Yes.</div></details></div>',
    );
});

test("leave a multiple accordion's items unnamed so several stay open", () => {
    const container = draw(() => (
        <Accordion type="multiple">
            <AccordionItem>
                <AccordionTrigger>Shipping</AccordionTrigger>
            </AccordionItem>
        </Accordion>
    ));
    expect(container.querySelector("details")?.hasAttribute("name")).toBe(false);
});
