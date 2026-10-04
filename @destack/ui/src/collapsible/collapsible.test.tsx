import { expect, test } from "@destack/test";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "./index.ts";
import { draw, markup } from "@destack/view/test";

test("toggle a collapsible from its summary", () => {
    const container = draw(() => (
        <Collapsible>
            <CollapsibleTrigger>3 more tags</CollapsibleTrigger>
            <CollapsibleContent>travel, food, family</CollapsibleContent>
        </Collapsible>
    ));
    container.querySelector("summary")?.click();
    expect(markup(container)).toBe(
        '<details data-slot="collapsible" open="">' +
            '<summary data-slot="collapsible-trigger">3 more tags</summary>' +
            '<div data-slot="collapsible-content">travel, food, family</div></details>',
    );
});
