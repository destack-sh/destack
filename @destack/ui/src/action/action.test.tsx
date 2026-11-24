import { argument, plural, t } from "@destack/locale";
import { defineAction, field } from "@destack/object";
import { schema } from "@destack/schema";
import { expect, onTestFinished, test, vi } from "@destack/test";
import { present } from "@destack/schema";
import { Button } from "../button/index.ts";
import { ActionRegistry, createSignal, flush, handle, handleAction, Show } from "@destack/view";
import { render, stubPopovers, wait } from "@destack/view/test";
import { Toaster } from "../toast/index.ts";
import { DropdownMenu, DropdownMenuContent, DropdownMenuTrigger } from "../dropdown-menu/index.ts";
import { ActionButton, ActionDialog, MenuAction } from "./index.ts";

/** Archive the note. */
const archive = defineAction({ name: "archive", title: t`Archive`, keybinding: "mod+e" });

/** Share the note with a person. */
const share = defineAction({
    name: "share",
    title: t`Share`,
    input: { person: field.string(schema.string()).labeled(t`Person`) },
});

/** Rename the note, and pick its color. */
const rename = defineAction({
    name: "rename",
    title: t`Rename`,
    input: {
        title: field.string(schema.string()).labeled(t`Title`),
        color: field.enum(["red", "blue"]).labeled(t`Color`),
        isShared: field.boolean().labeled(t`Shared`),
    },
});

/** Draw a note's buttons and the dialog that asks, recording what runs. */
function drawNote() {
    stubPopovers();
    const ran: unknown[] = [];
    const actions = new ActionRegistry();
    const { container } = render(
        () => {
            handleAction(archive, { run: () => void ran.push("archive") });
            handleAction(share, {
                choices: () => ["Ada", "Grace"].map((name) => ({ value: name, title: name })),
                run: ({ values }) => void ran.push(values),
            });
            handleAction(rename, { run: ({ values }) => void ran.push(values) });

            return (
                <>
                    <ActionButton action={archive} />
                    <ActionButton action={share} />
                    <ActionButton action={share} given={{ person: "Grace" }}>
                        Share with Grace
                    </ActionButton>
                    <ActionButton action={rename} />
                    <ActionDialog />
                </>
            );
        },
        { actions },
    );
    flush();

    return { container, ran };
}

/** Press the button with a title, and wait for what it asks. */
async function pressButton(container: HTMLElement, title: string): Promise<void> {
    [...container.querySelectorAll("button")].find((each) => each.textContent === title)?.click();
    await wait(0);
    flush();
}

test("run an action from its button, with its keys told and its given values", async () => {
    // press the two buttons that need nothing more
    const { container, ran } = drawNote();
    const [first] = container.querySelectorAll("button");
    await pressButton(container, "Archive");
    await pressButton(container, "Share with Grace");

    expect({ keys: first?.getAttribute("aria-keyshortcuts"), ran }).toEqual({
        keys: "Control+E",
        ran: ["archive", { person: "Grace" }],
    });
});

test("list the choices of the one input left, and run with the one picked", async () => {
    // share, and pick the second person
    const { container, ran } = drawNote();
    await pressButton(container, "Share");
    const items = [...document.querySelectorAll<HTMLElement>("[data-slot=command-item]")];
    const titles = items.map((item) => item.textContent);
    items[1]?.click();
    await wait(0);

    expect({ titles, ran }).toEqual({ titles: ["Ada", "Grace"], ran: [{ person: "Grace" }] });
});

test("ask for several inputs in a form, submitting once each required one is given", async () => {
    // give the title, then the color
    const { container, ran } = drawNote();
    await pressButton(container, "Rename");
    const form = document.querySelector("form");
    const submit = form?.querySelector<HTMLButtonElement>("button[type=submit]");
    const before = submit?.disabled;
    const title = form?.querySelector("input");
    const color = form?.querySelector("select");
    if (title === null || title === undefined || color === null || color === undefined) {
        throw new TypeError("the form has no title or color");
    }
    title.value = "Lisbon";
    title.dispatchEvent(new Event("input", { bubbles: true }));
    flush();
    const between = submit?.disabled;
    color.value = JSON.stringify("blue");
    color.dispatchEvent(new Event("change", { bubbles: true }));
    flush();
    submit?.click();
    await wait(0);

    expect({ before, between, ran }).toEqual({
        before: true,
        between: true,
        ran: [{ title: "Lisbon", color: "blue", isShared: false }],
    });
});

test("show the person an action that failed in the page's toasts", async () => {
    // press a button whose action throws, with the page's toaster
    stubPopovers();
    const reported: unknown[] = [];
    const report = vi.spyOn(globalThis, "reportError").mockImplementation((error) => {
        reported.push(error);
    });
    onTestFinished(() => report.mockRestore());
    const broken = new TypeError("broken");
    const { container } = render(() => {
        handleAction(archive, {
            run: () => {
                throw broken;
            },
        });

        return (
            <Toaster>
                <ActionButton action={archive} />
            </Toaster>
        );
    });
    flush();
    await pressButton(container, "Archive");
    const toasts = [...document.querySelectorAll("[data-slot=toast-title]")].map((each) =>
        (each.textContent ?? "").replaceAll(/[\u2066-\u2069]/gu, ""),
    );

    expect({ toasts, reported }).toEqual({
        toasts: ["“Archive” did not work"],
        reported: [broken],
    });
});

test("run nothing once the person cancels, and ask anew", async () => {
    // cancel the form, then open it again
    const { container, ran } = drawNote();
    await pressButton(container, "Rename");
    [...document.querySelectorAll("button")].find((each) => each.textContent === "Cancel")?.click();
    await wait(0);
    flush();
    const closed = document.querySelector("form")?.closest("dialog")?.open;
    await pressButton(container, "Rename");
    const isOpen = document.querySelector("form")?.closest("dialog")?.open;

    expect({ ran, closed, isOpen }).toEqual({ ran: [], closed: false, isOpen: true });
});

/** Pin the note. */
const pin = defineAction({ name: "note.pin", title: t`Pin` });

/** Set the note's color. */
const paint = defineAction({
    name: "note.paint",
    title: t`Color`,
    input: {
        color: field.enum({
            red: { title: t`Red`, color: "red" },
            blue: { title: t`Blue`, color: "blue" },
        }),
    },
});

/** Put the note a number of days off. */
const postpone = defineAction({
    name: "note.postpone",
    title: t`Postpone`,
    summary: t`Postpone by ${plural(argument("days"), { one: "# day", other: "# days" })}`,
    input: { days: field.integer() },
});

test("check what is on in a menu and on a button, open choices as a submenu, and say what a given value does", async () => {
    // draw a note's menu and its pin button
    stubPopovers();
    const ran: unknown[] = [];
    const [isPinned, setPinned] = createSignal(false);
    const { container } = render(() => {
        handleAction(pin, { isOn: isPinned, run: () => void setPinned((is) => !is) });
        handleAction(paint, { run: ({ values }) => void ran.push(values) });
        handleAction(postpone, { run: ({ values }) => void ran.push(values) });

        return (
            <>
                <ActionButton action={pin} />
                <DropdownMenu defaultOpen>
                    <DropdownMenuTrigger>Note</DropdownMenuTrigger>
                    <DropdownMenuContent>
                        <MenuAction action={pin} />
                        <MenuAction action={paint} />
                        <MenuAction action={postpone} given={{ days: 3 }} />
                    </DropdownMenuContent>
                </DropdownMenu>
            </>
        );
    });
    flush();
    await wait(0);
    flush();
    const read = () =>
        [...container.querySelectorAll<HTMLElement>("[data-menu-item]")].map((each) => [
            each.getAttribute("role"),
            (each.textContent ?? "").replaceAll(/[\u2066-\u2069]/gu, ""),
            each.getAttribute("aria-checked"),
        ]);
    const before = read();

    // pin by the button, pick blue from the submenu, and postpone by the given days
    const [button] = container.querySelectorAll("button");
    button?.click();
    flush();
    const pressed = button?.getAttribute("aria-pressed");
    const items = [...container.querySelectorAll<HTMLElement>("[data-menu-item]")];
    items.find((each) => each.textContent === "Blue")?.click();
    items.at(-1)?.click();
    await wait(0);

    expect({ before, pressed, ran }).toEqual({
        before: [
            ["menuitemcheckbox", "Pin", "false"],
            ["menuitem", "Color", null],
            ["menuitem", "Red", null],
            ["menuitem", "Blue", null],
            ["menuitem", "Postpone by 3 days", null],
        ],
        pressed: "true",
        ran: [{ color: "blue" }, { days: 3 }],
    });
});

test("enable a button that appears later where its action is handled around it", () => {
    // show the button inside an element that handles the action
    const ran: string[] = [];
    const [isShown, setShown] = createSignal(false, { ownedWrite: true });
    const { container } = render(
        () => (
            <section ref={handle(archive, { run: () => void ran.push("archive") })}>
                <Show when={isShown()}>
                    <ActionButton action={archive} />
                </Show>
            </section>
        ),
        { actions: new ActionRegistry() },
    );
    flush();
    setShown(true);
    flush();
    const button = container.querySelector("button");
    button?.click();
    flush();

    expect([button?.getAttribute("aria-disabled"), ran]).toEqual([null, ["archive"]]);
});

test("disable a button while its action does not run, the button staying in the tab order", () => {
    // enable the action, press, and disable it again
    const ran: string[] = [];
    const [isEnabled, setEnabled] = createSignal(false, { ownedWrite: true });
    const { container } = render(
        () => {
            handleAction(archive, { isEnabled, run: () => void ran.push("archive") });

            return <ActionButton action={archive} />;
        },
        { actions: new ActionRegistry() },
    );
    flush();
    const button = container.querySelector("button");
    const states = [false, true, false].map((next) => {
        setEnabled(next);
        flush();
        button?.click();
        flush();

        return [button?.getAttribute("aria-disabled"), button?.disabled, ran.length];
    });

    expect(states).toEqual([
        ["true", false, 0],
        [null, false, 1],
        ["true", false, 1],
    ]);
});

test("follow a link in place of its action", () => {
    const ran: string[] = [];
    const [isEnabled, setEnabled] = createSignal(true, { ownedWrite: true });
    const { container } = render(
        () => {
            handleAction(archive, { isEnabled, run: () => void ran.push("archive") });

            return <ActionButton action={archive} href="#archive" download="archive.zip" />;
        },
        { actions: new ActionRegistry() },
    );
    flush();
    const link = container.querySelector("a");
    const follow = () =>
        link?.dispatchEvent(new MouseEvent("click", { cancelable: true, bubbles: true }));
    const followed = follow();
    setEnabled(false);
    flush();

    expect({
        link: [link?.getAttribute("href"), link?.download, link?.target, link?.rel],
        followed: [followed, follow()],
        ran,
    }).toEqual({
        link: ["#archive", "archive.zip", "_blank", "noreferrer"],
        followed: [true, false],
        ran: [],
    });
});

test("style a button under its tooltip as any button", () => {
    const { container } = render(
        () => (
            <>
                <Button />
                <ActionButton action={archive} />
            </>
        ),
        { actions: new ActionRegistry() },
    );
    const [plain, action] = container.querySelectorAll("button");

    expect(action?.className).toBe(present(plain, "the plain button").className);
});
