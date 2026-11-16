import { OPEN_PATH } from "@destack/account/address";
import { present } from "@destack/schema";
import { CommandReference, Shortcut } from "@destack/object";
import { type CommandDeclaration, isCommandPlatform, Keybinding } from "./keybinding.ts";
import type { ActionEntry } from "./entry.ts";
import { PaletteMessage } from "./message.ts";
import { type ActionRegistry, redo, undo } from "../page/action.ts";
import type { Presentation } from "../browser/presentation.ts";
import { COMMAND_PARAMETER, FOCUS_PARAMETER } from "../declare/context.ts";
import { type Localization, t } from "@destack/locale";
import * as style from "@destack/style";
import { color, radius, shadow } from "@destack/theme/tokens.stylex";
import type { Navigation } from "@destack/environment/browser";

/** The event a standard action the desktop's Edit menu chooses arrives as, its name the detail. */
const ACTION_EVENT = "destack:action";

/** The standard actions the desktop's Edit menu runs, which a focused field runs itself. */
const EDIT_ACTIONS = [undo, redo];

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

/** The command palette over a page, and the one place the page's key combinations run. */
export class PaletteOverlay {
    /** The actions the page's components handle. */
    readonly #actions: ActionRegistry;
    /** The commands the page's package declares, with their declared key combinations. */
    readonly #commands: readonly CommandDeclaration[];
    /** The page's navigation, whose location the palette opens over. */
    readonly #navigation: Pick<Navigation, "currentEntry">;
    /** Read the page's localization now. */
    readonly #localization: () => Localization;
    /** Read the person's key combinations now, and whether they offer the package's commands. */
    readonly #presentation: () => Pick<Presentation, "keybindings" | "isOffered">;

    /** Serve the palette over a page with its actions and its package's commands. */
    constructor(options: {
        readonly actions: ActionRegistry;
        readonly commands: readonly CommandDeclaration[];
        readonly navigation: Pick<Navigation, "currentEntry">;
        readonly localization: () => Localization;
        readonly presentation: () => Pick<Presentation, "keybindings" | "isOffered">;
    }) {
        // set the actions, the commands, the navigation and what the page reads now
        this.#actions = options.actions;
        this.#commands = options.commands;
        this.#navigation = options.navigation;
        this.#localization = options.localization;
        this.#presentation = options.presentation;
    }

    /** Run what the person's keys bind until aborted: an action, else the palette at a command. */
    listen(signal: AbortSignal): void {
        // run each press the person's key combinations bind
        const isCommand = isCommandPlatform();
        const listener = (event: KeyboardEvent) => {
            // find the binding the keys run, leaving any other press to the page
            const binding = PaletteOverlay.#find(this.keybindings(), event, isCommand);
            if (binding === undefined) {
                return;
            }
            event.preventDefault();

            // run a handled action in the page, else open the palette at the command
            if (this.#actions.has(binding.command)) {
                void this.#actions
                    .run(binding.command)
                    .catch((error: unknown) => globalThis.reportError(error));
            } else {
                this.open(binding.command);
            }
        };
        window.addEventListener("keydown", listener, { signal });

        // run the Edit menu's undo or redo: the focused field's, else the page's
        const perform = (event: Event) => {
            const action = EDIT_ACTIONS.find(
                (each) => event instanceof CustomEvent && each.name === event.detail,
            );
            // refuse an action outside the Edit menu
            if (action === undefined) {
                throw new TypeError("the desktop chose an action outside the Edit menu");
            }
            // run the focused field's
            else if (PaletteOverlay.#isField(document.activeElement)) {
                PaletteOverlay.#edit(action.name);
            }
            // run the page's, leaving a disabled one to nothing as a greyed menu item does
            else if (this.#actions.has(action.reference)) {
                void this.#actions
                    .run(action.reference)
                    .catch((error: unknown) => globalThis.reportError(error));
            }
        };
        window.addEventListener(ACTION_EVENT, perform, { signal });
    }

    /** List the page's key combinations: its actions', then its package's offered commands'. */
    keybindings(): Keybinding[] {
        // bind each handled action, and each command while the person offers them
        const { keybindings: overrides, isOffered } = this.#presentation();
        const actions = this.#actions
            .actions()
            .map((action) =>
                Keybinding.of(
                    action.reference,
                    { key: action.keybinding, focus: action.focus },
                    overrides,
                ),
            );
        const commands = isOffered
            ? this.#commands.map((command) =>
                  Keybinding.of(command, { key: command.keybinding, focus: "page" }, overrides),
              )
            : [];

        return [...actions, ...commands].filter((binding) => binding !== undefined);
    }

    /** Open the palette over the page in a modal dialog framing its space's origin. */
    open(command?: CommandReference): void {
        // open the palette over this page through the open path, at a command when one is asked for
        const query = new URLSearchParams({ [FOCUS_PARAMETER]: this.#navigation.currentEntry.url });
        if (command !== undefined) {
            query.set(COMMAND_PARAMETER, CommandReference.format(command));
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
        // list the handled actions and the selection to the frame's origin
        else if (message.action === "actions.list") {
            const listed: PaletteMessage = {
                action: "actions.listed",
                actions: this.#entries(),
                selection: this.#actions.selection(),
            };
            const target = present(frame.contentWindow, "the palette's frame window");
            target.postMessage(listed, new URL(frame.src).origin);
        }
        // run an action once the palette closes, in the page the person acted on
        else if (message.action === "action.run") {
            dialog.close();
            await this.#actions.run(message.command);
        }
        // refuse a message only the page sends
        else {
            throw new TypeError(`the palette's frame sent '${message.action}'`);
        }
    }

    /** Find the binding a press runs, leaving a field's press to it unless bound anywhere. */
    static #find(
        bindings: readonly Keybinding[],
        event: KeyboardEvent,
        isCommand: boolean,
    ): Keybinding | undefined {
        const isField = PaletteOverlay.#isField(event.target);

        return bindings.find(
            (binding) =>
                (binding.focus === "any" || !isField) &&
                Shortcut.matches(binding.key, event, isCommand),
        );
    }

    /** Report whether a target is a field, which handles keys such as undo itself. */
    static #isField(target: EventTarget | null): boolean {
        return (
            target instanceof HTMLInputElement ||
            target instanceof HTMLTextAreaElement ||
            target instanceof HTMLSelectElement ||
            (target instanceof HTMLElement && target.isContentEditable)
        );
    }

    /** Run an editing command, such as undo, in the focused field. */
    static #edit(command: string): void {
        // oxlint-disable-next-line typescript/no-deprecated -- the editing command is the platform's only way to run a field's undo
        document.execCommand(command);
    }

    /** List the handled actions as the palette offers them, with the person's key combinations. */
    #entries(): ActionEntry[] {
        const localization = this.#localization();
        const bound = new Map(
            this.keybindings().map((binding) => [
                CommandReference.format(binding.command),
                binding.key,
            ]),
        );

        return this.#actions.actions().map((action) => ({
            kind: "action",
            packageId: action.package.id,
            name: action.reference.name,
            title: localization.render(action.title),
            category: action.category === undefined ? null : localization.render(action.category),
            keybinding: bound.get(CommandReference.format(action.reference)) ?? null,
        }));
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
