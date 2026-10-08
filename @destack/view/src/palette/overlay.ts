import { CommandReference } from "../declare/command.ts";
import { Accelerator, isCommandPlatform } from "./accelerator.ts";
import { COMMAND_PATH } from "../declare/context.ts";
import { type Localization, t } from "@destack/locale";
import { schema } from "@destack/schema";
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
export async function listenForCommands(
    signal: AbortSignal,
    localization: () => Localization,
): Promise<void> {
    // read the keybindings of the installation's commands
    const response = await fetch(`${COMMAND_PATH}/keybindings`, { signal });
    if (!response.ok) {
        throw new Error(`the command keybindings answered ${response.status}`);
    }
    const keybindings = Keybindings.parse(await response.json());
    const isCommand = isCommandPlatform();

    // open the palette, at a command its keybinding runs
    const listener = (event: KeyboardEvent) => {
        const bound = keybindings.find((entry) =>
            Accelerator.matches(entry.keybinding, event, isCommand),
        );
        if (bound !== undefined) {
            event.preventDefault();
            void openPalette(bound.command, localization()).catch(reportError);
        } else if (Accelerator.matches(PALETTE_ACCELERATOR, event, isCommand)) {
            event.preventDefault();
            void openPalette(undefined, localization()).catch(reportError);
        }
    };
    window.addEventListener("keydown", listener, { signal });
}

/** Open the command palette over the page in a modal dialog framing its space's origin, closing it on its message, Escape or a click beside it. */
async function openPalette(command: string | undefined, localization: Localization): Promise<void> {
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

    // show it in a modal dialog above the page
    const dialog = document.createElement("dialog");
    dialog.dataset["slot"] = "command-palette";
    const name = localization.render(t`Command palette`);
    dialog.setAttribute("aria-label", name);
    dialog.className = classOf(styles.dialog);
    const frame = document.createElement("iframe");
    frame.src = url;
    frame.title = name;
    frame.className = classOf(styles.frame);
    dialog.append(frame);
    document.body.append(dialog);
    dialog.showModal();

    // remove it once closed by its message, Escape or a click on the backdrop
    const origin = new URL(url).origin;
    const closing = new AbortController();
    dialog.addEventListener("close", () => {
        dialog.remove();
        closing.abort();
    });
    window.addEventListener(
        "message",
        (event) => {
            if (event.origin === origin && isClose(event.data)) {
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
function classOf(styled: style.Styles): string {
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
