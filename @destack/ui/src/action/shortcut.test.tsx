import { t } from "@destack/locale";
import { defineAction } from "@destack/object";
import { expect, test } from "@destack/test";
import { ActionRegistry, flush, handleOnPage } from "@destack/view";
import { render, stubPopovers } from "@destack/view/test";
import { ShortcutDialog, ShortcutHint, showShortcuts } from "./index.ts";

/** Go to the inbox. */
const inbox = defineAction({
    name: "inbox",
    title: t`Go to inbox`,
    category: t`Navigation`,
    keybinding: "g i",
});

/** Go to the drafts. */
const drafts = defineAction({
    name: "drafts",
    title: t`Go to drafts`,
    category: t`Navigation`,
    keybinding: "g d",
});

/** Write a note. */
const write = defineAction({ name: "write", title: t`Write`, keybinding: "c" });

/** Draw a page of actions with what shows their keys. */
function drawPage() {
    stubPopovers();
    const ran: string[] = [];
    const actions = new ActionRegistry();
    const { container } = render(
        () => {
            handleOnPage(inbox, { run: () => void ran.push("inbox") });
            handleOnPage(drafts, { run: () => void ran.push("drafts") });
            handleOnPage(write, { run: () => void ran.push("write") });

            return (
                <>
                    <ShortcutDialog />
                    <ShortcutHint />
                </>
            );
        },
        { actions },
    );
    flush();

    return { container, actions, ran };
}

/** Press a key on the page. */
function press(key: string): void {
    document.body.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true }));
    flush();
}

/** List the open dialog's sections: their titles, then their rows. */
function listed(): (string | undefined)[][] {
    return [...document.querySelectorAll("section")].map((section) => [
        section.querySelector("h3")?.textContent,
        ...[...section.querySelectorAll("div")].map((row) => row.textContent),
    ]);
}

test("show what can follow a typed stroke, and run what the next completes", () => {
    // type the first stroke, then the second
    const { container, ran } = drawPage();
    const hint = () => container.querySelector("[data-slot=shortcut-hint]");
    press("g");
    const shown = hint()?.textContent;
    press("d");

    expect({ shown, after: hint(), ran }).toEqual({
        shown: "IGo to inboxDGo to drafts",
        after: null,
        ran: ["drafts"],
    });
});

test("list the keys by category, and narrow them by words", async () => {
    // open it, then search
    const { actions } = drawPage();
    await actions.run(showShortcuts, null);
    flush();
    const all = listed();
    const search = document.querySelector("input");
    if (search === null) {
        throw new TypeError("the dialog has no search");
    }
    search.value = "draft";
    search.dispatchEvent(new Event("input", { bubbles: true }));
    flush();

    expect({ all, narrowed: listed() }).toEqual({
        all: [
            ["Navigation", "Go to inboxGthenI", "Go to draftsGthenD"],
            ["Help", "Keyboard shortcutsCtrl/"],
            ["Other", "WriteC"],
        ],
        narrowed: [["Navigation", "Go to draftsGthenD"]],
    });
});
