import { CommandReference } from "../declare/command.ts";
import { Accelerator } from "./keybinding.ts";
import { COMMAND_PATH } from "../declare/context.ts";
import { schema } from "@destack/schema";

/** The key combination opening the command palette over a view, as Linear and VS Code open theirs. */
const PALETTE_ACCELERATOR = "mod+k";

/** The palette's size over a view, in CSS pixels, as the desktop's floating palette takes it. */
const PALETTE_SIZE = { width: 680, height: 420 } as const;

/** The message the palette's page posts to close it. */
const CLOSE_ACTION = "palette.close";

/** The keybindings of the installation's commands, as the host resolves them for the person. */
const Keybindings = schema.array(
    schema.object({
        /** The command, as `<package>/<name>`. */
        command: CommandReference.key,
        /** Its key combination. */
        keybinding: Accelerator,
    }),
);

/** The location of a launched palette. */
const Launched = schema.object({
    /** The palette's location on the space's origin. */
    url: schema.url(),
});

/** Open the command palette over the page on its key combination, and on each command's own keybinding, until the signal aborts. */
export async function listenForCommands(signal: AbortSignal): Promise<void> {
    // read the keybindings of the installation's commands
    const response = await fetch(`${COMMAND_PATH}/keybindings`, { signal });
    const keybindings = response.ok ? Keybindings.parse(await response.json()) : [];
    const isMac = /Mac/u.test(navigator.userAgent);

    // open the palette, at a command its keybinding runs
    const listener = (event: KeyboardEvent) => {
        const bound = keybindings.find((entry) =>
            Accelerator.matches(entry.keybinding, event, isMac),
        );
        if (bound !== undefined) {
            event.preventDefault();
            void openPalette(bound.command).catch(reportError);
        } else if (Accelerator.matches(PALETTE_ACCELERATOR, event, isMac)) {
            event.preventDefault();
            void openPalette(undefined).catch(reportError);
        }
    };
    window.addEventListener("keydown", listener, { signal });
}

/** Open the command palette over the page in a frame of its space's origin, closing it on its message or a click beside it. */
async function openPalette(command: string | undefined): Promise<void> {
    // launch the palette over this page, at a command when one is asked for
    const response = await fetch(`${COMMAND_PATH}/launch`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
            focus: location.href,
            ...(command === undefined ? {} : { command }),
        }),
    });
    if (!response.ok) {
        throw new Error(await response.text());
    }
    const { url } = Launched.parse(await response.json());

    // float it above the page
    const backdrop = document.createElement("div");
    backdrop.dataset["slot"] = "command-palette";
    backdrop.style.cssText =
        "position:fixed;inset:0;display:grid;place-items:start center;padding-top:15vh;background:rgb(0 0 0 / 0.3);z-index:2147483647";
    const frame = document.createElement("iframe");
    frame.src = url;
    frame.style.cssText = `width:min(${PALETTE_SIZE.width}px,90vw);height:${PALETTE_SIZE.height}px;border:0;border-radius:12px;background:Canvas`;
    backdrop.append(frame);
    document.body.append(backdrop);

    // close it on its message or a click beside it
    const origin = new URL(url).origin;
    const closing = new AbortController();
    const close = () => {
        backdrop.remove();
        closing.abort();
    };
    window.addEventListener(
        "message",
        (event) => {
            if (event.origin === origin && isClose(event.data)) {
                close();
            }
        },
        { signal: closing.signal },
    );
    backdrop.addEventListener("click", (event) => {
        if (event.target === backdrop) {
            close();
        }
    });
    frame.focus();
}

/** Report whether a message asks to close the palette. */
function isClose(data: unknown): boolean {
    return (
        typeof data === "object" &&
        data !== null &&
        "action" in data &&
        data.action === CLOSE_ACTION
    );
}
