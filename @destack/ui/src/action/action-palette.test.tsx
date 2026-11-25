import { t } from "@destack/locale";
import { defineAction } from "@destack/object";
import { expect, test } from "@destack/test";
import { ActionRegistry, flush, handle, handleOnPage, palette } from "@destack/view";
import { render, stubPopovers, wait } from "@destack/view/test";
import { ActionPalette, selectAll } from "./index.ts";

/** Bold the text of an editor. */
const bold = defineAction({
    name: "bold",
    title: t`Bold`,
    category: t`Format`,
    keybinding: "mod+b",
});

/** Write a note. */
const write = defineAction({ name: "write", title: t`Write` });

test("list what runs where a person is, before what they open, and run it there", async () => {
    // open the palette in the editor, and run its first entry
    stubPopovers();
    const ran: unknown[] = [];
    const actions = new ActionRegistry();
    const open = { id: "open", title: "Open", run: () => void ran.push("open") };
    const { container } = render(
        () => {
            handleOnPage(selectAll, { run: () => void ran.push("select") });
            handleOnPage(write, { run: () => void ran.push("write") });

            return (
                <>
                    <textarea
                        ref={handle(bold, {
                            run: ({ element }) => void ran.push(element?.tagName),
                        })}
                    />
                    <ActionPalette
                        id="test"
                        title="Notes"
                        groups={[
                            {
                                id: "notes",
                                title: "Notes",
                                items: () => [{ id: "lisbon", title: "Lisbon", actions: [open] }],
                            },
                        ]}
                    />
                </>
            );
        },
        { actions },
    );
    flush();
    await actions.run(palette, container.querySelector("textarea"));
    flush();
    await wait(0);
    flush();
    const items = [...document.querySelectorAll<HTMLElement>("[data-slot=command-item]")];
    items[0]?.click();
    await wait(0);

    expect({ items: items.map((item) => item.textContent), ran }).toEqual({
        items: ["BoldFormatCtrlB", "Write", "Select allEditCtrlA", "Lisbon"],
        ran: ["TEXTAREA"],
    });
});
