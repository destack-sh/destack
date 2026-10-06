import { expect, test } from "@destack/test";
import { createSignal, flush } from "@destack/view";
import { Accordion, AccordionContent, AccordionItem, AccordionTrigger } from "./index.ts";
import { draw, markup } from "@destack/view/test";

/** List whether each item of a container is open, in order. */
function opened(container: Element): boolean[] {
    return [...container.querySelectorAll("details")].map((item) => item.open);
}

test("name a single accordion's items alike so the platform keeps one open", () => {
    const container = draw(() => (
        <Accordion defaultValue="styled">
            <AccordionItem value="accessible">
                <AccordionTrigger>Is it accessible?</AccordionTrigger>
                <AccordionContent>Yes.</AccordionContent>
            </AccordionItem>
            <AccordionItem value="styled">
                <AccordionTrigger>Is it styled?</AccordionTrigger>
                <AccordionContent>Yes.</AccordionContent>
            </AccordionItem>
        </Accordion>
    ));
    const chevron =
        '<span><svg viewBox="0 0 256 256" fill="currentColor" width="1em" height="1em" aria-hidden="true"></svg></span>';
    expect(markup(container)).toBe(
        '<div data-slot="accordion" data-orientation="vertical">' +
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
        <Accordion multiple>
            <AccordionItem value="shipping">
                <AccordionTrigger>Shipping</AccordionTrigger>
            </AccordionItem>
        </Accordion>
    ));
    expect(container.querySelector("details")?.hasAttribute("name")).toBe(false);
});

test("open the items a controlled accordion's owner names", () => {
    const [value, setValue] = createSignal<readonly string[]>(["shipping"]);
    const container = draw(() => (
        <Accordion multiple value={value()} onValueChange={setValue}>
            <AccordionItem value="shipping">
                <AccordionTrigger>Shipping</AccordionTrigger>
            </AccordionItem>
            <AccordionItem value="returns">
                <AccordionTrigger>Returns</AccordionTrigger>
            </AccordionItem>
        </Accordion>
    ));
    const before = opened(container);
    setValue(["returns"]);
    flush();
    expect([before, opened(container)]).toEqual([
        [true, false],
        [false, true],
    ]);
});

test("keep a disabled item closed when its trigger is clicked", () => {
    const container = draw(() => (
        <Accordion>
            <AccordionItem value="billing" disabled>
                <AccordionTrigger>Billing</AccordionTrigger>
            </AccordionItem>
        </Accordion>
    ));
    container.querySelector("summary")?.click();
    flush();
    expect([
        opened(container),
        container.querySelector("summary")?.getAttribute("aria-disabled"),
    ]).toEqual([[false], "true"]);
});
