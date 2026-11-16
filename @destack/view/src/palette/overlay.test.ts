import { Localization, SOURCE_LOCALE, t } from "@destack/locale";
import { PackageId } from "@destack/package";
import { CommandReference } from "@destack/object";
import { expect, onTestFinished, test } from "@destack/test";
import { defineAction } from "../declare/action.ts";
import { ActionRegistry, undo } from "../page/action.ts";
import { isCommandPlatform } from "./keybinding.ts";
import type { PaletteMessage } from "./message.ts";
import { PaletteOverlay } from "./overlay.ts";

/** The package serving the page. */
const NOTES = PackageId.parse("package-019f7480-0000-7000-8000-000000000001");

/** The note the person selected in the page. */
const NOTE = {
    packageId: NOTES,
    type: "note",
    scope: "space-019f7480-0000-7000-8000-000000000002",
    id: "note-019f7480-0000-7000-8000-000000000003",
};

/** Copy a link to the page. */
const copyLink = defineAction({
    name: "copy-link",
    title: t`Copy Link`,
    keybinding: "mod+shift+c",
});

/** Zoom into the page. */
const zoom = defineAction({
    name: "zoom",
    title: t`Zoom In`,
    category: t`View`,
    keybinding: "mod+shift+i",
});

/** Send the page's draft, from its field too. */
const send = defineAction({
    name: "send",
    title: t`Send`,
    keybinding: "mod+shift+s",
    focus: "any",
});

/** Open a palette over a page with a selected note, three actions and an archive command. */
function palette(overrides: Readonly<Record<string, string | null>> = {}) {
    // handle the actions and listen on the page
    const ran: string[] = [];
    const actions = new ActionRegistry();
    actions.handle(copyLink, { run: () => void ran.push("copy") });
    actions.handle(zoom, { run: () => void ran.push("zoom") });
    actions.handle(send, { run: () => void ran.push("send") });
    actions.select(() => [NOTE]);
    const overlay = new PaletteOverlay({
        actions,
        commands: [{ packageId: NOTES, name: "archive", keybinding: "mod+shift+a" }],
        navigation: {
            currentEntry: {
                url: "http://notes--personal--florian.localhost:7470/home",
                index: 0,
                state: null,
            },
        },
        localization: () => Localization.of(SOURCE_LOCALE, []),
        presentation: () => ({ keybindings: overrides, isOffered: true }),
    });
    const stopping = new AbortController();
    onTestFinished(() => stopping.abort());
    overlay.listen(stopping.signal);

    return { overlay, ran, stop: () => stopping.abort() };
}

/** Read the palette's dialog and frame from the document. */
function opened(): { readonly dialog: HTMLDialogElement; readonly frame: HTMLIFrameElement } {
    const dialog = document.querySelector("dialog[data-slot='command-palette']");
    const frame = dialog?.querySelector("iframe");
    if (!(dialog instanceof HTMLDialogElement) || !(frame instanceof HTMLIFrameElement)) {
        throw new TypeError("no palette is open");
    }
    onTestFinished(() => dialog.remove());

    return { dialog, frame };
}

/** Ask the page to run an action, as the palette's frame does. */
function run(action: typeof copyLink): PaletteMessage {
    return { action: "action.run", command: action.reference };
}

/** Choose undo in the desktop's Edit menu, which dispatches it to the focused page. */
function chooseUndo(): void {
    window.dispatchEvent(new CustomEvent("destack:action", { detail: "undo" }));
}

/** Send the page a message as a window would. */
function post(data: PaletteMessage, source: unknown): void {
    const event = new window.MessageEvent("message", { data });
    Object.defineProperty(event, "source", { value: source });
    window.dispatchEvent(event);
}

test("list the page's actions and its selection to the palette's frame, run the ones it chooses, and ignore other windows", () => {
    // open the palette and answer its frame's list request, recording what the page sends it
    const { overlay, ran } = palette();
    overlay.open();
    const { dialog, frame } = opened();
    const sent: unknown[] = [];
    const target = frame.contentWindow;
    if (target === null) {
        throw new TypeError("the palette's frame has no window");
    }
    target.postMessage = (message: unknown) => void sent.push(message);
    post({ action: "actions.list" }, target);

    // let another window ask to run one, then the frame
    post(run(copyLink), window);
    const before = [...ran];
    post(run(copyLink), target);

    // open the palette again and run the zoom action from its new frame
    overlay.open();
    const again = opened();
    post(run(zoom), again.frame.contentWindow);

    // expect the listing, nothing run for the other window, and the frame's run once the dialog closed
    const listed = (name: string, title: string, keybinding: string) => ({
        kind: "action",
        packageId: copyLink.package.id,
        name,
        title,
        category: null,
        keybinding,
    });
    expect({ sent, before, ran, isOpen: [dialog.isConnected, again.dialog.isConnected] }).toEqual({
        sent: [
            {
                action: "actions.listed",
                actions: [
                    listed("copy-link", "Copy Link", "mod+shift+c"),
                    { ...listed("zoom", "Zoom In", "mod+shift+i"), category: "View" },
                    listed("send", "Send", "mod+shift+s"),
                ],
                selection: [NOTE],
            },
        ],
        before: [],
        ran: ["copy", "zoom"],
        isOpen: [false, false],
    });
});

test("run an action on its keys, on the person's keys once rebound, from a field or editable text only when bound anywhere, and open the palette at a command", () => {
    // press keys on a target as the platform's mod key
    const isCommand = isCommandPlatform();
    const press = (key: string, target: EventTarget = window) =>
        target.dispatchEvent(
            new KeyboardEvent("keydown", {
                key,
                ctrlKey: !isCommand,
                metaKey: isCommand,
                shiftKey: true,
                bubbles: true,
            }),
        );
    const field = document.createElement("input");
    const editable = document.createElement("div");
    editable.contentEditable = "true";
    document.body.append(field, editable);
    onTestFinished(() => {
        field.remove();
        editable.remove();
    });

    // press the page's keys, then the person's over a page where they rebound copy
    const first = palette();
    press("c");
    press("c", field);
    press("c", editable);
    press("s", field);
    first.stop();
    const rebound = palette({ [CommandReference.format(copyLink.reference)]: "mod+shift+l" });
    press("l");
    const ranBefore = [...first.ran];

    // open the palette at the archive command
    press("a");
    const { frame } = opened();
    const command = new URL(frame.src).searchParams.get("command");

    expect({ first: ranBefore, rebound: rebound.ran, command }).toEqual({
        first: ["copy", "send"],
        rebound: ["copy"],
        command: `${NOTES}/archive`,
    });
});

test("run the Edit menu's undo as the page's action, and leave it to a focused field", () => {
    // handle the standard undo on the page, and focus a field for the second choice
    const ran: string[] = [];
    const actions = new ActionRegistry();
    actions.handle(undo, { run: () => void ran.push("undo") });
    const overlay = new PaletteOverlay({
        actions,
        commands: [],
        navigation: {
            currentEntry: { url: "http://notes.localhost:7470/home", index: 0, state: null },
        },
        localization: () => Localization.of(SOURCE_LOCALE, []),
        presentation: () => ({ keybindings: {}, isOffered: true }),
    });
    const stopping = new AbortController();
    onTestFinished(() => stopping.abort());
    overlay.listen(stopping.signal);

    // choose undo with the page focused, then with a field focused
    chooseUndo();
    const field = document.createElement("textarea");
    document.body.append(field);
    onTestFinished(() => field.remove());
    field.focus();
    chooseUndo();

    expect(ran).toEqual(["undo"]);
});
