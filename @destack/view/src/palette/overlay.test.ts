import { Localization, SOURCE_LOCALE, t } from "@destack/locale";
import { PackageId } from "@destack/package";
import { schema } from "@destack/schema";
import { expect, onTestFinished, test } from "@destack/test";
import { isCommandPlatform } from "./keybinding.ts";
import type { PaletteMessage } from "./message.ts";
import { PaletteOverlay } from "./overlay.ts";
import { CommandRegistry } from "./registry.ts";

/** The package serving the page. */
const NOTES = PackageId.parse("package-019f7480-0000-7000-8000-000000000001");

/** The note the person selected in the page. */
const NOTE = {
    packageId: NOTES,
    type: "note",
    scope: "space-019f7480-0000-7000-8000-000000000002",
    id: "note-019f7480-0000-7000-8000-000000000003",
};

/** Open a palette over a page that selects a note and registers copy and go-to-line commands. */
function palette(overrides: Readonly<Record<string, string | null>> = {}) {
    // register the command and listen on the page
    const ran: string[] = [];
    const commands = new CommandRegistry();
    commands.register({
        name: "copy-link",
        title: t`Copy Link`,
        keybinding: "mod+shift+c",
        run: () => void ran.push("copy"),
    });
    commands.register({
        name: "go-to-line",
        title: t`Go to Line…`,
        category: t`Editor`,
        input: schema.object({ line: schema.number().int().min(1) }),
        run: ({ line }) => void ran.push(`line ${line}`),
    });
    commands.select(() => [NOTE]);
    const overlay = new PaletteOverlay({
        commands,
        packageId: NOTES,
        navigation: {
            currentEntry: {
                url: "http://notes--personal--florian.localhost:7470/home",
                index: 0,
                state: null,
            },
        },
        localization: () => Localization.of(SOURCE_LOCALE, []),
        bindings: () => ({ keybindings: [], overrides }),
    });
    const stopping = new AbortController();
    onTestFinished(() => stopping.abort());
    overlay.listen(stopping.signal);

    return { overlay, ran };
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

/** Send the page a message as a window would. */
function post(data: PaletteMessage, source: unknown): void {
    const event = new window.MessageEvent("message", { data });
    Object.defineProperty(event, "source", { value: source });
    window.dispatchEvent(event);
}

test("list the view's commands with their input and its selection to the palette's frame, run the ones it chooses with their input, and ignore other windows", () => {
    // open the palette and answer its frame's list request, recording what the page sends it
    const { overlay, ran } = palette();
    overlay.open(undefined);
    const { dialog, frame } = opened();
    const sent: unknown[] = [];
    const target = frame.contentWindow;
    if (target === null) {
        throw new TypeError("the palette's frame has no window");
    }
    target.postMessage = (message: unknown) => void sent.push(message);
    post({ action: "commands.list" }, target);

    // let another window ask to run one, then the frame
    post({ action: "command.run", name: "copy-link", input: {} }, window);
    const before = [...ran];
    post({ action: "command.run", name: "copy-link", input: {} }, target);

    // open the palette again and run the line command from its new frame
    overlay.open(undefined);
    const again = opened();
    post(
        { action: "command.run", name: "go-to-line", input: { line: 3 } },
        again.frame.contentWindow,
    );

    // expect the listing, nothing run for the other window, and the frame's run once the dialog closed
    expect({ sent, before, ran, isOpen: [dialog.isConnected, again.dialog.isConnected] }).toEqual({
        sent: [
            {
                action: "commands.listed",
                packageId: NOTES,
                commands: [
                    { name: "copy-link", title: "Copy Link", keybinding: "mod+shift+c" },
                    {
                        name: "go-to-line",
                        title: "Go to Line…",
                        category: "Editor",
                        input: {
                            $schema: "https://json-schema.org/draft/2020-12/schema",
                            type: "object",
                            properties: {
                                line: { type: "integer", minimum: 1, maximum: 9007199254740991 },
                            },
                            required: ["line"],
                            additionalProperties: false,
                        },
                    },
                ],
                selection: [NOTE],
            },
        ],
        before: [],
        ran: ["copy", "line 3"],
        isOpen: [false, false],
    });
});

test("run a registered command on its keys in the page, on the person's keys once they rebind it", () => {
    // press the view's keys, then the person's keys over a page where they rebound it
    const isCommand = isCommandPlatform();
    const press = (key: string) =>
        window.dispatchEvent(
            new KeyboardEvent("keydown", {
                key,
                ctrlKey: !isCommand,
                metaKey: isCommand,
                shiftKey: true,
            }),
        );
    const declared = palette();
    press("c");
    const rebound = palette({ [`${NOTES}/copy-link`]: "mod+shift+l" });
    press("l");

    // expect the first page's command run on its keys, and the rebound one on the person's
    expect([declared.ran, rebound.ran]).toEqual([["copy"], ["copy"]]);
});
