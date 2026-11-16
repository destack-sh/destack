import { t } from "@destack/locale";
import { expect, onTestFinished, test } from "@destack/test";
import { createSignal, flush } from "solid-js";
import { defineAction } from "../declare/action.ts";
import { ActionRegistry } from "./action.ts";

/** Bold the selected text of an editor. */
const bold = defineAction({
    name: "bold",
    title: t`Bold`,
    category: t`Format`,
    keybinding: "mod+b",
});

test("run an action by the handler around the focus, else the page's, and only while enabled", async () => {
    // handle bold in two editors, the inner one only with a selection, and on the page
    const ran: string[] = [];
    const [hasSelection, setSelection] = createSignal(false);
    const outer = document.createElement("section");
    const inner = document.createElement("textarea");
    const other = document.createElement("textarea");
    outer.append(inner);
    document.body.append(outer, other);
    onTestFinished(() => {
        outer.remove();
        other.remove();
    });
    const actions = new ActionRegistry();
    actions.handle(bold, { run: () => void ran.push("page") });
    actions.handle(bold, { run: () => void ran.push("outer"), within: () => outer });
    const stopInner = actions.handle(bold, {
        run: () => void ran.push("inner"),
        within: () => inner,
        isEnabled: hasSelection,
    });

    // run with focus outside both, in the inner editor without and with a selection, then once it stops
    const press = async (focused: HTMLElement) => {
        focused.focus();
        await actions.run(bold.reference);
    };
    await press(other);
    await press(inner);
    setSelection(true);
    flush();
    await press(inner);
    stopInner();
    await press(inner);

    expect(ran).toEqual(["page", "outer", "inner", "outer"]);
});

test("list and offer only the actions that run now, refusing one none runs", async () => {
    // handle bold on the page while enabled
    const [isEnabled, setEnabled] = createSignal(true);
    const actions = new ActionRegistry();
    actions.handle(bold, { run: () => {}, isEnabled });

    // read it enabled, then disabled
    const listed = actions.actions().map((action) => action.name);
    setEnabled(false);
    flush();
    const refused = await actions.run(bold.reference).catch((error: unknown) => String(error));

    expect({
        listed,
        has: actions.has(bold.reference),
        after: actions.actions(),
        refused,
    }).toEqual({
        listed: ["bold"],
        has: false,
        after: [],
        refused: `TypeError: the page runs no action '${bold.package.id}/bold' now`,
    });
});
