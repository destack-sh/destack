import { expect, test } from "@destack/test";
import { flush } from "@destack/view";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "./index.ts";
import { markup, render } from "@destack/view/test";

test("toggle a collapsible from its summary", () => {
    const { container } = render(() => (
        <Collapsible>
            <CollapsibleTrigger>3 more tags</CollapsibleTrigger>
            <CollapsibleContent>travel, food, family</CollapsibleContent>
        </Collapsible>
    ));
    container.querySelector("summary")?.click();
    flush();
    expect(markup(container)).toBe(
        '<details data-slot="collapsible" data-state="open" open="">' +
            '<summary data-slot="collapsible-trigger" data-state="open">3 more tags</summary>' +
            '<div data-slot="collapsible-content" data-state="open">travel, food, family</div></details>',
    );
});
