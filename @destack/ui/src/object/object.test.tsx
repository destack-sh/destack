import { expect, test } from "@destack/test";
import { render } from "@destack/view/test";
import { ObjectChip } from "./object.tsx";

test("draw an object's chip as a link with its icon, title and accent, and as text without an address", () => {
    const linked = render(() => (
        <ObjectChip title="Write docs" glyph={{ emoji: "📝" }} color="cyan" href="/task/task-1" />
    ));
    const plain = render(() => <ObjectChip title="Launch" />);
    const link = linked.container.querySelector("[data-slot=object-chip]");
    const text = plain.container.querySelector("[data-slot=object-chip]");

    expect({
        link: [link?.tagName, link?.getAttribute("href"), link?.textContent, link?.children.length],
        text: [text?.tagName, text?.getAttribute("href"), text?.textContent, text?.children.length],
    }).toEqual({
        link: ["A", "/task/task-1", "📝Write docs", 3],
        text: ["SPAN", null, "Launch", 1],
    });
});
