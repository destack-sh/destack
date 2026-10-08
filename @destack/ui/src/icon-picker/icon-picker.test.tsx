import { expect, test } from "@destack/test";
import { draw } from "@destack/view/test";
import { flush } from "@destack/view";
import { IconPicker, IconPickerContent, IconPickerSearch } from "./index.ts";

/** Wait until a container shows an element a selector finds, flushing between looks. */
async function shown(container: HTMLElement, selector: string): Promise<HTMLElement> {
    for (let attempt = 0; attempt < 100; attempt++) {
        flush();
        const found = container.querySelector<HTMLElement>(selector);
        if (found !== null) {
            return found;
        }
        await new Promise((resolve) => {
            setTimeout(resolve, 10);
        });
    }
    throw new TypeError(`nothing shows ${selector}`);
}

test("pick an icon a search finds by one of its words, under its category's heading", async () => {
    // search savings, which finds the acorn among others
    const picked: string[] = [];
    const container = draw(() => (
        <IconPicker search="savings" onPick={(icon) => picked.push(icon)}>
            <IconPickerSearch />
            <IconPickerContent />
        </IconPicker>
    ));
    (await shown(container, '[role=gridcell][aria-label="acorn"]')).click();

    expect({
        picked,
        heading: container.querySelector("[data-slot=grid-list-section-header]")?.textContent,
    }).toEqual({ picked: ["acorn"], heading: "Commerce" });
});
