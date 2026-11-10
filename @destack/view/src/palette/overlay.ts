import { OPEN_PATH } from "@destack/account/address";
import type { PackageId } from "@destack/package";
import { present } from "@destack/schema";
import { Shortcut, CommandReference } from "@destack/object";
import { isCommandPlatform, type Keybinding } from "./keybinding.ts";
import { Palette } from "./entry.ts";
import { PaletteMessage } from "./message.ts";
import type { CommandRegistry, RegisteredCommand } from "./registry.ts";
import { COMMAND_PARAMETER, FOCUS_PARAMETER } from "../declare/context.ts";
import { type Localization, t } from "@destack/locale";
import * as style from "@destack/style";
import { color, radius, shadow } from "@destack/theme/tokens.stylex";
import type { Navigation } from "@destack/environment/browser";

/** The key combination opening the command palette over a view. */
const PALETTE_SHORTCUT = "mod+k";

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

/** What a page's keybindings resolve from: the declared commands' and the person's overrides. */
export interface PaletteBindings {
    /** The key combinations running the declared commands, for the person. */
    readonly keybindings: readonly Keybinding[];
    /** The person's key combinations by command key, null for one turned off. */
    readonly overrides: Readonly<Record<string, Shortcut | null>>;
}

/** The command palette over a page: its keybindings, its dialog and the messages of its frame. */
export class PaletteOverlay {
    /** The commands the page's view registers. */
    readonly #commands: CommandRegistry;
    /** The package serving the page, whose registered commands the person may rebind. */
    readonly #packageId: PackageId;
    /** The page's navigation, whose location the palette opens over. */
    readonly #navigation: Pick<Navigation, "currentEntry">;
    /** Read the page's localization now. */
    readonly #localization: () => Localization;
    /** Read the page's keybindings now. */
    readonly #bindings: () => PaletteBindings;

    /** Serve the palette over a page of a package's view. */
    constructor(options: {
        readonly commands: CommandRegistry;
        readonly packageId: PackageId;
        readonly navigation: Pick<Navigation, "currentEntry">;
        readonly localization: () => Localization;
        readonly bindings: () => PaletteBindings;
    }) {
        // set the commands, the package, the navigation and what the page reads now
        this.#commands = options.commands;
        this.#packageId = options.packageId;
        this.#navigation = options.navigation;
        this.#localization = options.localization;
        this.#bindings = options.bindings;
    }

    /** Run a registered command or open the palette on the person's keys until aborted. */
    listen(signal: AbortSignal): void {
        const isCommand = isCommandPlatform();
        const listener = (event: KeyboardEvent) => {
            // read what the keys run: a registered command, a declared one, or the palette
            const registered = this.#registered().find(
                (entry) =>
                    entry.keybinding !== undefined &&
                    Shortcut.matches(entry.keybinding, event, isCommand),
            );
            const declared = this.#bindings().keybindings.find((entry) =>
                Shortcut.matches(entry.keybinding, event, isCommand),
            );
            const isPalette = Shortcut.matches(PALETTE_SHORTCUT, event, isCommand);

            // run a registered command taking no input in the page
            if (registered !== undefined && registered.input === undefined) {
                event.preventDefault();
                void this.#commands
                    .run(registered.name, {})
                    .catch((error: unknown) => globalThis.reportError(error));
            }
            // open the palette at a registered command, which asks for its input
            else if (registered !== undefined) {
                event.preventDefault();
                this.open(
                    CommandReference.format({ packageId: this.#packageId, name: registered.name }),
                );
            }
            // open the palette at a declared command, which it runs with its input
            else if (declared !== undefined) {
                event.preventDefault();
                this.open(declared.command);
            }
            // open the palette
            else if (isPalette) {
                event.preventDefault();
                this.open(undefined);
            }
        };
        window.addEventListener("keydown", listener, { signal });
    }

    /** Open the palette over the page in a modal dialog framing its space's origin. */
    open(command: string | undefined): void {
        // open the palette over this page through the open path, at a command when one is asked for
        const query = new URLSearchParams({ [FOCUS_PARAMETER]: this.#navigation.currentEntry.url });
        if (command !== undefined) {
            query.set(COMMAND_PARAMETER, command);
        }
        const url = `${OPEN_PATH}?${query.toString()}`;

        // show it in a modal dialog above the page
        const localization = this.#localization();
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

        // answer the frame's messages until the dialog closes on Escape, a click on the backdrop or its message
        const closing = new AbortController();
        dialog.addEventListener("close", () => {
            dialog.remove();
            closing.abort();
        });
        window.addEventListener(
            "message",
            (event) => {
                if (event.source === frame.contentWindow) {
                    void this.#answer(event.data, frame, dialog).catch((error: unknown) =>
                        globalThis.reportError(error),
                    );
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

    /** Answer one message of the palette's frame. */
    async #answer(
        data: unknown,
        frame: HTMLIFrameElement,
        dialog: HTMLDialogElement,
    ): Promise<void> {
        // close the palette
        const message = PaletteMessage.parse(data);
        if (message.action === "palette.close") {
            dialog.close();
        }
        // list the registered commands to the frame's origin
        else if (message.action === "commands.list") {
            const listed: PaletteMessage = {
                action: "commands.listed",
                packageId: this.#packageId,
                commands: this.#registered(),
                selection: this.#commands.selection(),
            };
            const target = present(frame.contentWindow, "the palette's frame window");
            target.postMessage(listed, new URL(frame.src).origin);
        }
        // run a registered command once the palette closes, in the page the person acted on
        else if (message.action === "command.run") {
            dialog.close();
            await this.#commands.run(message.name, message.input);
        }
        // refuse a message only the page sends
        else {
            throw new TypeError(`the palette's frame sent ${message.action}`);
        }
    }

    /** List the registered commands with the key combinations the person gives them. */
    #registered(): RegisteredCommand[] {
        const { overrides } = this.#bindings();

        return this.#commands
            .list(this.#localization())
            .map(({ keybinding: given, ...command }) => {
                // take the person's key combination over the view's
                const reference = { packageId: this.#packageId, name: command.name };
                const keybinding = Palette.keybinding(reference, given, overrides);

                return { ...command, ...(keybinding === null ? {} : { keybinding }) };
            });
    }
}

/** Read the class names of a StyleX style, which always compiles to some. */
function readClass(styled: style.Styles): string {
    const { class: names } = style.attrs(styled);
    if (names === undefined) {
        throw new TypeError("a palette style compiled to no class");
    }

    return names;
}
