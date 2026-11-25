import { OriginPath } from "@destack/package";
import { DeclarationReference } from "@destack/package/declare";
import { present } from "@destack/schema";
import { type ActionDeclaration, Keybinding } from "./keybinding.ts";
import type { ActionEntry } from "./entry.ts";
import { PaletteMessage } from "./message.ts";
import { type ActionRegistry, redo, undo } from "../page/action.ts";
import type { Presentation } from "../browser/presentation.ts";
import { ACTION_PARAMETER, FOCUS_PARAMETER } from "../declare/context.ts";
import { type Localization, Shortcut, t } from "@destack/locale";
import * as style from "@destack/style";
import { color, radius, shadow, stroke } from "@destack/theme/tokens.stylex";
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
        borderWidth: stroke.border,
        borderStyle: "solid",
        borderColor: color.border,
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
    /** The method actions the page's package declares, with their declared key combinations. */
    readonly #declared: readonly ActionDeclaration[];
    /** The page's navigation, whose location the palette opens over. */
    readonly #navigation: Pick<Navigation, "currentEntry">;
    /** Read the page's localization now. */
    readonly #localization: () => Localization;
    /** Read the person's key combinations now, and whether they offer the package's actions. */
    readonly #presentation: () => Pick<Presentation, "keybindings" | "isOffered">;

    /** Serve the palette over a page with its handled actions and its package's declared ones. */
    constructor(options: {
        readonly actions: ActionRegistry;
        readonly declared: readonly ActionDeclaration[];
        readonly navigation: Pick<Navigation, "currentEntry">;
        readonly localization: () => Localization;
        readonly presentation: () => Pick<Presentation, "keybindings" | "isOffered">;
    }) {
        // set the handled and declared actions, the navigation and what the page reads now
        this.#actions = options.actions;
        this.#declared = options.declared;
        this.#navigation = options.navigation;
        this.#localization = options.localization;
        this.#presentation = options.presentation;
    }

    /** Run what the person's keys bind until aborted: a handled action, else the palette at it. */
    listen(signal: AbortSignal): void {
        // run each press the person's key combinations bind
        const isCommand = Shortcut.isCommand(navigator.userAgent);
        const listener = (event: KeyboardEvent) => {
            // run a handled action where the keys were pressed, else open the palette at the action
            const focus = event.target instanceof Element ? event.target : null;
            const isField = PaletteOverlay.#isField(event.target);
            const bindings = this.keybindings(focus, isCommand)
                .filter(({ focus: allowed }) => allowed === "any" || !isField)
                .map(({ key, action }) => ({ key, target: action }));
            this.#actions.type(event, bindings, { isCommand }, (reference) => {
                const bound = this.#actions.find(reference);
                if (bound !== undefined && this.#actions.has(reference, focus)) {
                    this.#actions.start(bound, focus);
                } else {
                    this.open(focus, reference);
                }
            });
        };
        window.addEventListener("keydown", listener, { signal });

        // run the Edit menu's undo or redo where the platform's focus is: the field's, else the page's
        const perform = (event: Event) => {
            const focus = document.activeElement;
            const action = EDIT_ACTIONS.find(
                (each) => event instanceof CustomEvent && each.name === event.detail,
            );
            // refuse an action outside the Edit menu
            if (action === undefined) {
                throw new TypeError("the desktop chose an action outside the Edit menu");
            }
            // run the focused field's
            else if (PaletteOverlay.#isField(focus)) {
                PaletteOverlay.#edit(action.name);
            }
            // run the page's, leaving a disabled one to nothing as a greyed menu item does
            else if (this.#actions.has(action.reference, focus)) {
                void this.#actions
                    .run(action, focus)
                    .catch((error: unknown) => globalThis.reportError(error));
            }
        };
        window.addEventListener(ACTION_EVENT, perform, { signal });
    }

    /** List the key combinations at a focus: its handled actions', then the package's offered. */
    keybindings(focus: Element | null, isCommand: boolean): Keybinding[] {
        // bind each handled action, and each declared one while the person offers them
        const { keybindings: overrides, isOffered } = this.#presentation();
        const actions = this.#actions
            .actions(focus)
            .flatMap((action) => Keybinding.of(action.reference, action, overrides, isCommand));
        const declared = isOffered
            ? this.#declared.flatMap((action) => {
                  const { keybinding } = action.description;

                  return Keybinding.of(action, { keybinding, focus: "page" }, overrides, isCommand);
              })
            : [];

        return [...actions, ...declared];
    }

    /** Open the palette over a focus in a modal dialog framing its space's origin. */
    open(focus: Element | null, action?: DeclarationReference): void {
        // open the palette over this page through the open path, at an action when one is asked for
        const query = new URLSearchParams({ [FOCUS_PARAMETER]: this.#navigation.currentEntry.url });
        if (action !== undefined) {
            query.set(ACTION_PARAMETER, DeclarationReference.key(action));
        }
        const url = `${OriginPath.open}?${query.toString()}`;

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
                    void this.#answer(event.data, frame, dialog, focus).catch((error: unknown) =>
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
        focus: Element | null,
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
                actions: this.#entries(focus),
                selection: this.#actions.selection(focus),
            };
            const target = present(frame.contentWindow, "the palette's frame window");
            target.postMessage(listed, new URL(frame.src).origin);
        }
        // run an action once the palette closes, where the person was when it opened
        else if (message.action === "action.run") {
            dialog.close();
            const action = present(
                this.#actions.find(message.reference),
                `the action ${message.reference.name}`,
            );
            this.#actions.start(action, focus);
        }
        // refuse a message only the page sends
        else {
            throw new TypeError(`the palette's frame sent '${message.action}'`);
        }
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

    /** List the actions handled at a focus as the palette offers them, with their keys. */
    #entries(focus: Element | null): ActionEntry[] {
        // find the keys there
        const localization = this.#localization();
        const isCommand = Shortcut.isCommand(navigator.userAgent);
        const bindings = this.keybindings(focus, isCommand);

        // offer each by the first of its keys
        const bound = new Map(
            bindings.toReversed().map(({ action, key }) => [DeclarationReference.key(action), key]),
        );

        return this.#actions.actions(focus).map((action) => ({
            kind: "action",
            packageId: action.package.id,
            name: action.reference.name,
            title: localization.render(action.title),
            category: action.category === undefined ? null : localization.render(action.category),
            keybinding: bound.get(DeclarationReference.key(action.reference)) ?? null,
            call: null,
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
