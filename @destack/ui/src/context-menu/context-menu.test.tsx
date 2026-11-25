import { t } from "@destack/locale";
import { defineAction } from "@destack/object";
import { expect, test } from "@destack/test";
import { ActionRegistry, flush, handleOnPage } from "@destack/view";
import { render, stubPopovers } from "@destack/view/test";
import { MenuAction } from "../menu/index.ts";
import { ContextMenu, ContextMenuContent, ContextMenuTrigger } from "./index.ts";

/** Open what the person is on. */
const open = defineAction({ name: "open", title: t`Open` });

test("run an action on the element the menu opened on", () => {
    // right-click the second of two rows, and its text
    stubPopovers();
    const ran: (string | null | undefined)[] = [];
    const { container } = render(
        () => {
            handleOnPage(open, { run: ({ element }) => void ran.push(element?.id) });

            return (
                <ContextMenu>
                    <ContextMenuTrigger id="area">
                        <div id="first" tabindex={-1}>
                            One
                        </div>
                        <div id="second" tabindex={-1}>
                            <b>Two</b>
                        </div>
                        <i>Three</i>
                    </ContextMenuTrigger>
                    <ContextMenuContent>
                        <MenuAction action={open} />
                    </ContextMenuContent>
                </ContextMenu>
            );
        },
        { actions: new ActionRegistry() },
    );
    flush();
    for (const tag of ["b", "i"]) {
        container
            .querySelector(tag)
            ?.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true }));
        flush();
        document.querySelector<HTMLElement>("[role=menuitem]")?.click();
        flush();
    }

    expect(ran).toEqual(["second", "area"]);
});
