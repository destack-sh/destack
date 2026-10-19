import { Accelerator, isCommandPlatform, type Keybinding } from "./accelerator.ts";
import { COMMAND_PARAMETER, FOCUS_PARAMETER, OPEN_PATH } from "../declare/context.ts";
import { type Localization, t } from "@destack/locale";
import * as style from "@destack/style";
import { color, radius, shadow } from "@destack/theme/tokens.stylex";

/** The key combination opening the command palette over a view. */
const PALETTE_ACCELERATOR = "mod+k";

/** The palette's size over a view, in CSS pixels, as the desktop's floating palette takes it. */
const PALETTE_SIZE = { width: 680, height: 420 } as const;

/** The styles of the palette's dialog and frame over a view. */
const styles = style.create({
    dialog: {
        width: `min(${PALETTE_SIZE.width}px, 90vw)`,
        height: `${PALETTE_SIZE.height}px`,
        maxWidth: "none",
        marginBlockStart: "15vh",
        padding: 0,
        borderWidth: 0,
        borderRadius: radius[5],
        overflow: "hidden",
        backgroundColor: "Canvas",
        boxShadow: shadow.overlay,
        "::backdrop": {
            backgroundColor: color.scrim,
        },
    },
    frame: {
        width: "100%",
        height: "100%",
        borderWidth: 0,
    },
});

/** The message the palette's page posts to close it. */
const CLOSE_ACTION = "palette.close";

/** Open the command palette over the page on its key combination until the signal aborts. */
export function listenForCommands(
    signal: AbortSignal,
    localization: () => Localization,
    keybindings: () => readonly Keybinding[],
): void {
    // open the palette, at a command its keybinding runs
    const isCommand = isCommandPlatform();
    const listener = (event: KeyboardEvent) => {
        const bound = keybindings().find((entry) =>
            Accelerator.matches(entry.keybinding, event, isCommand),
        );
        if (bound !== undefined) {
            event.preventDefault();
            openPalette(bound.command, localization());
        } else if (Accelerator.matches(PALETTE_ACCELERATOR, event, isCommand)) {
            event.preventDefault();
            openPalette(undefined, localization());
        }
    };
    window.addEventListener("keydown", listener, { signal });
}

/** Open the command palette over the page in a modal dialog that frames its space's origin. */
function openPalette(command: string | undefined, localization: Localization): void {
    // open the palette over this page through the open path, at a command when one is asked for
    const query = new URLSearchParams({ [FOCUS_PARAMETER]: location.href });
    if (command !== undefined) {
        query.set(COMMAND_PARAMETER, command);
    }
    const url = `${OPEN_PATH}?${query.toString()}`;

    // show it in a modal dialog above the page
    const dialog = document.createElement("dialog");
    dialog.dataset["slot"] = "command-palette";
    const name = localization.render(t`Command palette`);
    dialog.setAttribute("aria-label", name);
    dialog.className = readClass(styles.dialog);
    const frame = document.createElement("iframe");
    frame.src = url;
    frame.title = name;
    frame.className = readClass(styles.frame);
    dialog.append(frame);
    document.body.append(dialog);
    dialog.showModal();

    // remove it once closed by its frame's message, Escape or a click on the backdrop
    const closing = new AbortController();
    dialog.addEventListener("close", () => {
        dialog.remove();
        closing.abort();
    });
    window.addEventListener(
        "message",
        (event) => {
            if (event.source === frame.contentWindow && isClose(event.data)) {
                dialog.close();
            }
        },
        { signal: closing.signal },
    );
    dialog.addEventListener("click", (event) => {
        if (event.target === dialog) {
            dialog.close();
        }
    });
    frame.focus();
}

/** Read the class names of a StyleX style, which always compiles to some. */
function readClass(styled: style.Styles): string {
    const { class: names } = style.attrs(styled);
    if (names === undefined) {
        throw new TypeError("a palette style compiled to no class");
    }

    return names;
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
